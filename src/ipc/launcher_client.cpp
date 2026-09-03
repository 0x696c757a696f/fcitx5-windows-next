#include "launcher_client.h"

#include "protocol_ffi.h"

#include <Windows.h>

#include <utility>
#include <vector>

namespace fcitx::windows::ipc {
namespace {

// Launcher control uses the same bounded client buffer as the previous
// adapter. Rust protocol-core remains the codec and validation authority.
constexpr std::size_t kHeaderSize = 64;
constexpr std::size_t kMaxFrameSize = 256U * 1024U;
constexpr std::uint16_t kLauncherResponseType = 6;
constexpr std::uint32_t kStartDemandCommand = 1;

struct Fcitx5WindowsCommonPipeTransactWithError {
    std::uint8_t status;
    std::uint32_t failureError;
    std::size_t responseLen;
};
struct Fcitx5WindowsCommonLauncherResponseScalarInput {
    std::uint64_t requestId;
    std::uint64_t responseTo;
    std::uint64_t engineEpoch;
    std::uint32_t sessionId;
    std::uint64_t contextId;
    std::uint64_t compositionId;
    std::uint64_t revision;
    std::uint32_t status;
    std::uint32_t launcherState;
    std::uint32_t engineState;
    std::uint32_t startDisposition;
    std::uint8_t safeMode;
    std::uint64_t retryAfterMilliseconds;
    std::uint64_t expectedRequestId;
    std::uint32_t expectedSessionId;
};
struct Fcitx5WindowsCommonLauncherResponseScalars {
    std::uint8_t status;
    std::uint32_t responseStatus;
    std::uint32_t launcherState;
    std::uint32_t engineState;
    std::uint32_t startDisposition;
    std::uint8_t safeMode;
    std::uint64_t requestId;
    std::uint64_t responseTo;
    std::uint64_t engineEpoch;
    std::uint32_t sessionId;
    std::uint64_t contextId;
    std::uint64_t compositionId;
    std::uint64_t revision;
    std::uint64_t retryAfterMilliseconds;
};
extern "C" Fcitx5WindowsCommonPipeTransactWithError
fcitx5_windows_common_pipe_transact_with_error(
    void* pipe, const std::uint8_t* request, std::size_t request_len,
    std::uint8_t* response_output, std::size_t response_capacity, std::uint64_t deadline);
extern "C" void* fcitx5_windows_common_open_pipe_client_utf16(
    const std::uint16_t* pipe_name, std::size_t pipe_name_len, std::uint64_t deadline,
    std::uint8_t wait_when_busy);
extern "C" void fcitx5_windows_common_close_pipe_client(void* pipe);
extern "C" std::uint8_t fcitx5_windows_common_deadline_has_time(std::uint64_t deadline);
extern "C" Fcitx5WindowsCommonLauncherResponseScalars
fcitx5_windows_common_apply_launcher_response_scalars(
    Fcitx5WindowsCommonLauncherResponseScalarInput input);
extern "C" std::uint64_t fcitx5_windows_common_next_launcher_request_id();
extern "C" std::uint8_t fcitx5_windows_common_ipc_status_ok(std::uint32_t status);
extern "C" void fcitx5_windows_common_set_last_error(std::uint32_t error);

const std::uint16_t* wideData(std::wstring_view value) noexcept {
    static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
    return reinterpret_cast<const std::uint16_t*>(value.data());
}

std::string_view byteView(FcitxBytesC value) noexcept {
    if (value.len == 0 || value.data == nullptr) return {};
    return {reinterpret_cast<const char*>(value.data), value.len};
}

struct FrameView {
    FcitxMetadataC metadata{};
    const std::uint8_t* body{};
    std::size_t bodyLength{};
};

bool decodeFrame(const std::vector<std::uint8_t>& bytes, FrameView& frame) noexcept {
    if (bytes.size() < kHeaderSize || bytes.size() > kMaxFrameSize) return false;
    std::uint16_t type = 0;
    std::uint32_t bodySize = 0;
    if (fcitx5_protocol_core_decode_header(bytes.data(), kHeaderSize, &type, &bodySize,
                                           &frame.metadata) == 0 ||
        type != kLauncherResponseType || bodySize != bytes.size() - kHeaderSize) {
        return false;
    }
    frame.body = bytes.data() + kHeaderSize;
    frame.bodyLength = bodySize;
    return true;
}

template <typename Message, typename Decoder>
bool decodeMessage(const std::vector<std::uint8_t>& bytes, Message& message,
                   std::vector<std::uint8_t>& strings, Decoder decoder) noexcept {
    FrameView frame;
    if (!decodeFrame(bytes, frame)) return false;
    std::size_t stringsNeeded = 0;
    if (decoder(&frame.metadata, frame.body, frame.bodyLength, &message, nullptr, 0,
                &stringsNeeded) != 0) {
        message.metadata = frame.metadata;
        return true;
    }
    if (stringsNeeded == 0 || stringsNeeded > kMaxFrameSize) return false;
    try {
        strings.assign(stringsNeeded, 0);
    } catch (...) {
        return false;
    }
    if (decoder(&frame.metadata, frame.body, frame.bodyLength, &message, strings.data(),
                strings.size(), &stringsNeeded) == 0) {
        return false;
    }
    message.metadata = frame.metadata;
    return true;
}

template <typename Message, typename Encoder>
bool encodeMessage(const Message& message, std::vector<std::uint8_t>& bytes,
                   Encoder encoder) noexcept {
    try {
        bytes.assign(kMaxFrameSize, 0);
        std::size_t length = 0;
        if (encoder(&message, bytes.data(), bytes.size(), &length) == 0 ||
            length < kHeaderSize || length > bytes.size()) {
            return false;
        }
        bytes.resize(length);
        return true;
    } catch (...) {
        bytes.clear();
        return false;
    }
}

} // namespace

bool sendLauncherCommand(const platform::RuntimeIdentity& identity,
                         std::uint64_t absoluteDeadlineMilliseconds,
                         const PeerPolicy& peerPolicy, std::uint32_t command,
                         LauncherResponse& response) noexcept {
    return sendLauncherCommand(identity, platform::currentRuntimeGeneration(),
                               absoluteDeadlineMilliseconds, peerPolicy, command, response);
}

bool sendLauncherCommand(const platform::RuntimeIdentity& identity,
                         std::wstring_view generation,
                         std::uint64_t absoluteDeadlineMilliseconds,
                         const PeerPolicy& peerPolicy, std::uint32_t command,
                         LauncherResponse& response) noexcept {
    response = {};
    if (!identity.mayUseUserEngine() ||
        fcitx5_windows_common_deadline_has_time(absoluteDeadlineMilliseconds) == 0) {
        return false;
    }
    const std::wstring endpoint = platform::makeLocalEndpointName(identity, generation, L"launcher");
    HANDLE pipe = fcitx5_windows_common_open_pipe_client_utf16(
        wideData(endpoint), endpoint.size(), absoluteDeadlineMilliseconds, 0);
    if (pipe == INVALID_HANDLE_VALUE) return false;
    bool success = verifyPipeServer(pipe, identity, peerPolicy);
    DWORD failure = success ? ERROR_SUCCESS : ERROR_ACCESS_DENIED;
    try {
        const auto requestId = fcitx5_windows_common_next_launcher_request_id();
        const FcitxLauncherRequestC request{
            FcitxMetadataC{requestId, 0, 0, identity.sessionId, 0, 0, 0}, command};
        std::vector<std::uint8_t> requestBytes;
        if (!encodeMessage(request, requestBytes, fcitx5_protocol_core_encode_launcher_request)) {
            success = false;
            failure = ERROR_INVALID_DATA;
        } else if (!success) {
            failure = ERROR_ACCESS_DENIED;
        } else {
            std::vector<std::uint8_t> responseBytes(kMaxFrameSize, 0);
            const auto transferred = fcitx5_windows_common_pipe_transact_with_error(
                pipe, requestBytes.data(), requestBytes.size(), responseBytes.data(),
                responseBytes.size(), absoluteDeadlineMilliseconds);
            if (transferred.status == 0) {
                success = false;
                failure = transferred.failureError == ERROR_SUCCESS ? ERROR_TIMEOUT
                                                                    : transferred.failureError;
            } else {
                responseBytes.resize(transferred.responseLen);
                FcitxLauncherResponseC decoded{};
                std::vector<std::uint8_t> strings;
                success = decodeMessage(responseBytes, decoded, strings,
                                         fcitx5_protocol_core_decode_launcher_response);
                if (success) {
                    const auto scalars = fcitx5_windows_common_apply_launcher_response_scalars(
                        Fcitx5WindowsCommonLauncherResponseScalarInput{
                            decoded.metadata.requestId,
                            decoded.metadata.responseTo,
                            decoded.metadata.engineEpoch,
                            decoded.metadata.sessionId,
                            decoded.metadata.contextId,
                            decoded.metadata.compositionId,
                            decoded.metadata.revision,
                            decoded.status,
                            decoded.launcherState,
                            decoded.engineState,
                            decoded.startDisposition,
                            decoded.safeMode,
                            decoded.retryAfterMilliseconds,
                            requestId,
                            identity.sessionId});
                    success = scalars.status != 0;
                    if (success) {
                        response.requestId = scalars.requestId;
                        response.responseTo = scalars.responseTo;
                        response.engineEpoch = scalars.engineEpoch;
                        response.sessionId = scalars.sessionId;
                        response.contextId = scalars.contextId;
                        response.compositionId = scalars.compositionId;
                        response.revision = scalars.revision;
                        response.status = scalars.responseStatus;
                        response.launcherState = scalars.launcherState;
                        response.engineState = scalars.engineState;
                        response.startDisposition = scalars.startDisposition;
                        response.safeMode = scalars.safeMode != 0;
                        response.retryAfterMilliseconds = scalars.retryAfterMilliseconds;
                        response.currentInputMethodId.assign(byteView(decoded.currentInputMethodId));
                        response.currentInputMethodName.assign(byteView(decoded.currentInputMethodName));
                        response.currentInputMethodNativeName.assign(
                            byteView(decoded.currentInputMethodNativeName));
                        response.currentInputMethodShortLabel.assign(
                            byteView(decoded.currentInputMethodShortLabel));
                    }
                }
                if (!success) failure = ERROR_INVALID_DATA;
            }
        }
    } catch (...) {
        success = false;
        failure = ERROR_NOT_ENOUGH_MEMORY;
    }
    fcitx5_windows_common_close_pipe_client(pipe);
    fcitx5_windows_common_set_last_error(success ? ERROR_SUCCESS : failure);
    return success;
}

bool requestLauncherStart(const platform::RuntimeIdentity& identity,
                          std::uint64_t absoluteDeadlineMilliseconds,
                          const PeerPolicy& peerPolicy) noexcept {
    return requestLauncherStart(identity, platform::currentRuntimeGeneration(),
                                absoluteDeadlineMilliseconds, peerPolicy);
}

bool requestLauncherStart(const platform::RuntimeIdentity& identity,
                          std::wstring_view generation,
                          std::uint64_t absoluteDeadlineMilliseconds,
                          const PeerPolicy& peerPolicy) noexcept {
    LauncherResponse response;
    return sendLauncherCommand(identity, generation, absoluteDeadlineMilliseconds, peerPolicy,
                               kStartDemandCommand, response) &&
           fcitx5_windows_common_ipc_status_ok(response.status) != 0;
}

} // namespace fcitx::windows::ipc
