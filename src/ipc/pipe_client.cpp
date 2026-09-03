#include "pipe_client.h"

#include "launcher_client.h"

#include <vector>

namespace fcitx::windows::ipc {
namespace {

// These bounds are the Rust protocol-core hot-frame contract. The client only
// owns transport buffering; Rust owns frame validation and byte layout.
constexpr std::size_t kHeaderSize = 64;
constexpr std::size_t kMaxFrameSize = 256U * 1024U;
constexpr std::uint16_t kHelloResponseType = 2;
constexpr std::uint16_t kKeyResponseType = 4;
constexpr std::uint16_t kCandidateSelectResponseType = 8;
constexpr std::uint16_t kEngineStatusResponseType = 11;

struct Fcitx5WindowsCommonPipeTransact {
    std::uint8_t status;
    std::size_t responseLen;
};
extern "C" Fcitx5WindowsCommonPipeTransact fcitx5_windows_common_pipe_transact(
    void* pipe, const std::uint8_t* request, std::size_t request_len,
    std::uint8_t* response_output, std::size_t response_capacity, std::uint64_t deadline);
extern "C" std::uint64_t fcitx5_windows_common_deadline_after_milliseconds(
    std::uint32_t milliseconds);
extern "C" std::uint32_t fcitx5_windows_common_current_process_id();
extern "C" std::uint64_t fcitx5_windows_common_next_pipe_client_request_id();
extern "C" void* fcitx5_windows_common_open_pipe_client_utf16(
    const std::uint16_t* pipe_name, std::size_t pipe_name_len, std::uint64_t deadline,
    std::uint8_t wait_when_busy);
extern "C" void fcitx5_windows_common_close_pipe_client(void* pipe);

struct Fcitx5WindowsCommonUtf8ToWide {
    std::uint8_t status;
    std::size_t utf16Len;
};
struct Fcitx5WindowsCommonUtf8OffsetToWide {
    std::uint8_t status;
    std::uint32_t utf16Offset;
};
struct Fcitx5WindowsCommonHelloResponseScalars {
    std::uint8_t status;
    std::uint8_t handshakeComplete;
    std::uint64_t engineEpoch;
};
struct Fcitx5WindowsCommonKeyResponseScalarInput {
    std::uint64_t responseTo;
    std::uint64_t engineEpoch;
    std::uint32_t sessionId;
    std::uint64_t contextId;
    std::uint64_t compositionId;
    std::uint64_t revision;
    std::uint32_t status;
    std::uint64_t expectedRequestId;
    std::uint64_t expectedEngineEpoch;
    std::uint32_t expectedSessionId;
    std::uint64_t expectedContextId;
    std::uint64_t previousRevision;
    std::uint8_t handled;
    std::uint32_t selectedCandidate;
    std::uint32_t candidatePage;
    std::uint32_t candidateTotal;
    std::uint8_t candidateVisibility;
    std::uint8_t deleteSurroundingText;
    std::int32_t deleteSurroundingOffset;
    std::uint32_t deleteSurroundingSize;
    std::uint8_t forwardKey;
    std::uint32_t forwardKeySym;
    std::uint32_t forwardKeyStates;
    std::int32_t forwardKeyCode;
    std::uint8_t forwardKeyRelease;
    std::uint8_t caretValid;
    std::int32_t caretLeft;
    std::int32_t caretTop;
    std::int32_t caretRight;
    std::int32_t caretBottom;
    std::uint32_t caretDpi;
};
struct Fcitx5WindowsCommonKeyResponseScalars {
    std::uint8_t status;
    std::uint8_t handled;
    std::uint8_t deleteSurroundingText;
    std::uint8_t forwardKey;
    std::uint8_t forwardKeyRelease;
    std::uint8_t caretValid;
    std::uint64_t engineEpoch;
    std::uint64_t contextCompositionId;
    std::uint64_t contextRevision;
    std::uint64_t resultCompositionId;
    std::uint64_t resultRevision;
    std::uint32_t selectedCandidate;
    std::uint32_t candidatePage;
    std::uint32_t candidateTotal;
    std::int32_t deleteSurroundingOffset;
    std::uint32_t deleteSurroundingSize;
    std::uint32_t forwardKeySym;
    std::uint32_t forwardKeyStates;
    std::int32_t forwardKeyCode;
    std::int32_t caretLeft;
    std::int32_t caretTop;
    std::int32_t caretRight;
    std::int32_t caretBottom;
    std::uint32_t caretDpi;
    std::uint8_t candidateVisibility;
};
struct Fcitx5WindowsCommonEngineStatusResponseScalarInput {
    std::uint64_t requestId;
    std::uint64_t responseTo;
    std::uint64_t engineEpoch;
    std::uint32_t sessionId;
    std::uint64_t contextId;
    std::uint64_t compositionId;
    std::uint64_t revision;
    std::uint32_t status;
    std::uint64_t expectedRequestId;
    std::uint64_t expectedEngineEpoch;
    std::uint32_t expectedSessionId;
};
struct Fcitx5WindowsCommonEngineStatusResponseScalars {
    std::uint8_t status;
    std::uint32_t responseStatus;
    std::uint64_t requestId;
    std::uint64_t responseTo;
    std::uint64_t engineEpoch;
    std::uint32_t sessionId;
    std::uint64_t contextId;
    std::uint64_t compositionId;
    std::uint64_t revision;
};
extern "C" Fcitx5WindowsCommonUtf8ToWide fcitx5_windows_common_utf8_to_wide_utf16(
    const std::uint8_t* input, std::size_t input_len, std::uint16_t* output,
    std::size_t capacity);
extern "C" Fcitx5WindowsCommonUtf8OffsetToWide
fcitx5_windows_common_utf8_offset_to_wide(const std::uint8_t* input, std::size_t input_len,
                                          std::uint32_t offset);
extern "C" Fcitx5WindowsCommonKeyResponseScalars
fcitx5_windows_common_apply_key_response_scalars(
    Fcitx5WindowsCommonKeyResponseScalarInput input);
extern "C" Fcitx5WindowsCommonHelloResponseScalars
fcitx5_windows_common_apply_hello_response_scalars(
    std::uint64_t response_to, std::uint64_t engine_epoch, std::uint32_t session_id,
    std::uint32_t status, std::uint64_t expected_request_id,
    std::uint32_t expected_session_id);
extern "C" std::uint8_t fcitx5_windows_common_accept_candidate_select_response(
    std::uint64_t response_to, std::uint64_t engine_epoch, std::uint32_t session_id,
    std::uint64_t context_id, std::uint64_t revision, std::uint32_t status,
    std::uint64_t expected_request_id, std::uint64_t expected_engine_epoch,
    std::uint32_t expected_session_id, std::uint64_t expected_context_id,
    std::uint64_t previous_revision);
extern "C" std::uint8_t fcitx5_windows_common_accept_candidate_select_request(
    std::uint64_t current_engine_epoch, std::uint64_t expected_engine_epoch,
    std::uint32_t target_process_id, std::uint64_t context_id, std::uint64_t composition_id,
    std::uint64_t revision, std::uint64_t candidate_id);
extern "C" Fcitx5WindowsCommonEngineStatusResponseScalars
fcitx5_windows_common_apply_engine_status_response_scalars(
    Fcitx5WindowsCommonEngineStatusResponseScalarInput input);

const std::uint16_t* wideData(std::wstring_view value) noexcept {
    static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
    return reinterpret_cast<const std::uint16_t*>(value.data());
}

const std::uint8_t* byteData(std::string_view value) noexcept {
    return value.empty() ? nullptr : reinterpret_cast<const std::uint8_t*>(value.data());
}

std::string_view byteView(FcitxBytesC value) noexcept {
    if (value.len == 0 || value.data == nullptr) return {};
    return {reinterpret_cast<const char*>(value.data), value.len};
}

std::uint8_t flagByte(bool value) noexcept { return value ? 1 : 0; }

struct FrameView {
    FcitxMetadataC metadata{};
    const std::uint8_t* body{};
    std::size_t bodyLength{};
};

bool decodeFrame(const std::vector<std::uint8_t>& bytes, std::uint16_t expectedType,
                 FrameView& frame) noexcept {
    if (bytes.size() < kHeaderSize || bytes.size() > kMaxFrameSize) return false;
    std::uint16_t type = 0;
    std::uint32_t bodySize = 0;
    if (fcitx5_protocol_core_decode_header(bytes.data(), kHeaderSize, &type, &bodySize,
                                           &frame.metadata) == 0 ||
        type != expectedType || bodySize != bytes.size() - kHeaderSize) {
        return false;
    }
    frame.body = bytes.data() + kHeaderSize;
    frame.bodyLength = bodySize;
    return true;
}

template <typename Message, typename Decoder>
bool decodeMessage(const std::vector<std::uint8_t>& bytes, std::uint16_t expectedType,
                   Message& message, std::vector<std::uint8_t>& strings,
                   Decoder decoder) noexcept {
    FrameView frame;
    if (!decodeFrame(bytes, expectedType, frame)) return false;
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

bool decodeKeyResponse(const std::vector<std::uint8_t>& bytes, FcitxKeyResponseC& message,
                       std::vector<std::uint8_t>& strings,
                       std::vector<FcitxCandidateRecordC>& candidates) noexcept {
    FrameView frame;
    if (!decodeFrame(bytes, kKeyResponseType, frame)) return false;
    std::size_t stringsNeeded = 0;
    std::size_t candidatesNeeded = 0;
    if (fcitx5_protocol_core_decode_key_response(
            &frame.metadata, frame.body, frame.bodyLength, &message, nullptr, 0, &stringsNeeded,
            nullptr, 0, &candidatesNeeded) != 0) {
        message.metadata = frame.metadata;
        return true;
    }
    if ((stringsNeeded == 0 && candidatesNeeded == 0) || stringsNeeded > kMaxFrameSize ||
        candidatesNeeded > kMaxFrameSize) {
        return false;
    }
    try {
        strings.assign(stringsNeeded, 0);
        candidates.assign(candidatesNeeded, FcitxCandidateRecordC{});
    } catch (...) {
        return false;
    }
    if (fcitx5_protocol_core_decode_key_response(
            &frame.metadata, frame.body, frame.bodyLength, &message,
            strings.empty() ? nullptr : strings.data(), strings.size(), &stringsNeeded,
            candidates.empty() ? nullptr : candidates.data(), candidates.size(),
            &candidatesNeeded) == 0) {
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

bool utf8ToWide(std::string_view input, std::wstring& output) {
    output.clear();
    const auto query = fcitx5_windows_common_utf8_to_wide_utf16(
        byteData(input), input.size(), nullptr, 0);
    if (query.status == 0) return false;
    output.assign(query.utf16Len, L'\0');
    const auto filled = fcitx5_windows_common_utf8_to_wide_utf16(
        byteData(input), input.size(), output.empty() ? nullptr
                                                       : reinterpret_cast<std::uint16_t*>(output.data()),
        output.size());
    if (filled.status == 0 || filled.utf16Len != output.size()) {
        output.clear();
        return false;
    }
    return true;
}

bool utf8OffsetToWide(std::string_view input, std::uint32_t offset,
                      std::uint32_t& output) {
    const auto converted = fcitx5_windows_common_utf8_offset_to_wide(
        byteData(input), input.size(), offset);
    if (converted.status == 0) return false;
    output = converted.utf16Offset;
    return true;
}

} // namespace

PipeClient::PipeClient()
    : launcherGeneration_(platform::currentRuntimeGeneration()),
      peerPolicy_(PeerPolicy::development()) {
    if (platform::queryCurrentIdentity(identity_)) {
        pipeName_ = platform::makeLocalEndpointName(identity_, L"engine");
        sessionId_ = identity_.sessionId;
    }
}

PipeClient::PipeClient(std::wstring pipeName, PeerPolicy peerPolicy,
                       std::wstring launcherGeneration)
    : pipeName_(std::move(pipeName)),
      launcherGeneration_(launcherGeneration.empty() ? platform::currentRuntimeGeneration()
                                                     : std::move(launcherGeneration)),
      peerPolicy_(std::move(peerPolicy)) {
    if (platform::queryCurrentIdentity(identity_)) sessionId_ = identity_.sessionId;
}

PipeClient::~PipeClient() { disconnect(); }

void PipeClient::disconnect() noexcept {
    if (pipe_ != INVALID_HANDLE_VALUE) {
        fcitx5_windows_common_close_pipe_client(pipe_);
        pipe_ = INVALID_HANDLE_VALUE;
    }
    handshakeComplete_ = false;
    engineEpoch_ = 0;
    contexts_.clear();
}

bool PipeClient::connect(std::uint64_t deadline) noexcept {
    if (pipe_ != INVALID_HANDLE_VALUE) return true;
    if (pipeName_.empty() || !identity_.mayUseUserEngine()) return false;
    pipe_ = fcitx5_windows_common_open_pipe_client_utf16(
        wideData(pipeName_), pipeName_.size(), deadline, 1);
    if (pipe_ == INVALID_HANDLE_VALUE) return false;
    if (!verifyPipeServer(pipe_, identity_, peerPolicy_)) {
        disconnect();
        return false;
    }
    return true;
}

bool PipeClient::transact(const std::vector<std::uint8_t>& request,
                          std::vector<std::uint8_t>& response,
                          std::uint64_t deadline) noexcept {
    if (request.empty() || request.size() > kMaxFrameSize) {
        disconnect();
        return false;
    }
    try {
        response.assign(kMaxFrameSize, 0);
    } catch (...) {
        disconnect();
        return false;
    }
    const auto transferred = fcitx5_windows_common_pipe_transact(
        pipe_, request.data(), request.size(), response.data(), response.size(), deadline);
    if (transferred.status == 0 || transferred.responseLen < kHeaderSize ||
        transferred.responseLen > response.size()) {
        disconnect();
        return false;
    }
    try {
        response.resize(transferred.responseLen);
    } catch (...) {
        disconnect();
        return false;
    }
    return true;
}

bool PipeClient::handshake(std::uint64_t deadline) noexcept {
    try {
        if (handshakeComplete_) return true;
        if (sessionId_ == 0) return false;
        const auto requestId = fcitx5_windows_common_next_pipe_client_request_id();
        const FcitxHelloRequestC request{
            FcitxMetadataC{requestId, 0, 0, sessionId_, 0, 0, 0},
            static_cast<std::uint32_t>(sizeof(void*) * 8),
            fcitx5_windows_common_current_process_id()};
        std::vector<std::uint8_t> requestBytes;
        if (!encodeMessage(request, requestBytes, fcitx5_protocol_core_encode_hello_request))
            return false;
        std::vector<std::uint8_t> responseBytes;
        if (!transact(requestBytes, responseBytes, deadline)) return false;
        FcitxHelloResponseC response{};
        std::vector<std::uint8_t> strings;
        if (!decodeMessage(responseBytes, kHelloResponseType, response, strings,
                           fcitx5_protocol_core_decode_hello_response)) {
            disconnect();
            return false;
        }
        const auto scalars = fcitx5_windows_common_apply_hello_response_scalars(
            response.metadata.responseTo, response.metadata.engineEpoch,
            response.metadata.sessionId, response.status, requestId, sessionId_);
        if (scalars.status == 0) {
            disconnect();
            return false;
        }
        engineEpoch_ = scalars.engineEpoch;
        handshakeComplete_ = scalars.handshakeComplete != 0;
        return true;
    } catch (...) {
        disconnect();
        return false;
    }
}

bool PipeClient::acceptKeyResponse(const FcitxKeyResponseC& response,
                                   std::uint64_t requestId,
                                   std::uint64_t contextId,
                                   ContextState& contextState,
                                   KeyResult& result) noexcept {
    const auto scalars = fcitx5_windows_common_apply_key_response_scalars(
        Fcitx5WindowsCommonKeyResponseScalarInput{
            response.metadata.responseTo, response.metadata.engineEpoch,
            response.metadata.sessionId, response.metadata.contextId,
            response.metadata.compositionId, response.metadata.revision, response.status,
            requestId, engineEpoch_, sessionId_, contextId, contextState.revision,
            response.handled, response.selectedCandidate, response.candidatePage,
            response.candidateTotal, response.candidateVisibility,
            response.deleteSurroundingText, response.deleteSurroundingOffset,
            response.deleteSurroundingSize, response.forwardKey, response.forwardKeySym,
            response.forwardKeyStates, response.forwardKeyCode, response.forwardKeyRelease,
            response.caret.valid, response.caret.left, response.caret.top, response.caret.right,
            response.caret.bottom, response.caret.dpi});
    const std::string_view commit = byteView(response.commit);
    const std::string_view preedit = byteView(response.preedit);
    if (scalars.status == 0 || !utf8ToWide(commit, result.commit) ||
        !utf8ToWide(preedit, result.preedit) ||
        !utf8OffsetToWide(preedit, response.preeditCaretUtf8, result.preeditCaretUtf16)) {
        return false;
    }
    contextState.compositionId = scalars.contextCompositionId;
    contextState.revision = scalars.contextRevision;
    result.engineEpoch = scalars.engineEpoch;
    result.compositionId = scalars.resultCompositionId;
    result.revision = scalars.resultRevision;
    result.handled = scalars.handled != 0;
    result.selectedCandidate = scalars.selectedCandidate;
    result.candidatePage = scalars.candidatePage;
    result.candidateTotal = scalars.candidateTotal;
    result.candidateVisibility = scalars.candidateVisibility;
    result.deleteSurroundingText = scalars.deleteSurroundingText != 0;
    result.deleteSurroundingOffset = scalars.deleteSurroundingOffset;
    result.deleteSurroundingSize = scalars.deleteSurroundingSize;
    result.forwardKey = scalars.forwardKey != 0;
    result.forwardKeySym = scalars.forwardKeySym;
    result.forwardKeyStates = scalars.forwardKeyStates;
    result.forwardKeyCode = scalars.forwardKeyCode;
    result.forwardKeyRelease = scalars.forwardKeyRelease != 0;
    result.caret = CaretRect{scalars.caretValid != 0, scalars.caretLeft, scalars.caretTop,
                             scalars.caretRight, scalars.caretBottom, scalars.caretDpi};
    if (response.candidateCount > 0 && response.candidates == nullptr) return false;
    result.candidates.reserve(response.candidateCount);
    for (std::size_t index = 0; index < response.candidateCount; ++index) {
        const auto& source = response.candidates[index];
        KeyResult::Candidate candidate;
        candidate.id = source.id;
        if (!utf8ToWide(byteView(source.label), candidate.label) ||
            !utf8ToWide(byteView(source.text), candidate.text) ||
            !utf8ToWide(byteView(source.comment), candidate.comment)) {
            result = {};
            return false;
        }
        result.candidates.emplace_back(std::move(candidate));
    }
    return true;
}

bool PipeClient::processKey(std::uint64_t contextId, std::uint32_t virtualKey,
                            std::uint32_t keyFlags, KeyResult& result,
                            const CaretRect& caret, bool popupAllowed,
                            std::uint32_t scanCode, bool extendedKey,
                            std::uint64_t keyboardLayout, std::string_view logicalText,
                            std::string_view inputMethod, bool surroundingTextValid,
                            std::string_view surroundingText, std::uint32_t surroundingCursor,
                            std::uint32_t surroundingAnchor) noexcept {
    result = {};
    try {
        const auto existingContext = contexts_.find(contextId);
        const bool newContext = existingContext == contexts_.end();
        const std::uint64_t deadline = fcitx5_windows_common_deadline_after_milliseconds(
            newContext ? kContextStartDeadlineMilliseconds : kInputDeadlineMilliseconds);
        if (!connect(deadline)) {
            (void)requestLauncherStart(identity_, launcherGeneration_, deadline,
                                       PeerPolicy::development());
        }
        if (!connect(deadline) || !handshake(deadline)) {
            disconnect();
            return false;
        }
        const auto requestId = fcitx5_windows_common_next_pipe_client_request_id();
        auto contextState = newContext ? ContextState{} : existingContext->second;
        const FcitxKeyRequestC request{
            FcitxMetadataC{requestId, 0, engineEpoch_, sessionId_, contextId,
                           contextState.compositionId, contextState.revision},
            virtualKey,
            keyFlags,
            scanCode,
            flagByte(extendedKey),
            flagByte(popupAllowed),
            keyboardLayout,
            FcitxBytesC{byteData(logicalText), logicalText.size()},
            FcitxBytesC{byteData(inputMethod), inputMethod.size()},
            flagByte(surroundingTextValid),
            FcitxBytesC{byteData(surroundingText), surroundingText.size()},
            surroundingCursor,
            surroundingAnchor,
            FcitxCaretRectC{flagByte(caret.valid), caret.left, caret.top, caret.right,
                            caret.bottom, caret.dpi}};
        std::vector<std::uint8_t> requestBytes;
        if (!encodeMessage(request, requestBytes, fcitx5_protocol_core_encode_key_request))
            return false;
        std::vector<std::uint8_t> responseBytes;
        if (!transact(requestBytes, responseBytes, deadline)) return false;
        FcitxKeyResponseC response{};
        std::vector<std::uint8_t> strings;
        std::vector<FcitxCandidateRecordC> candidates;
        if (!decodeKeyResponse(responseBytes, response, strings, candidates) ||
            !acceptKeyResponse(response, requestId, contextId, contextState, result)) {
            disconnect();
            return false;
        }
        contexts_[contextId] = contextState;
        return true;
    } catch (...) {
        disconnect();
        result.handled = false;
        result.commit.clear();
        return false;
    }
}

bool PipeClient::selectCandidate(std::uint32_t targetProcessId,
                                 std::uint64_t expectedEngineEpoch,
                                 std::uint64_t contextId, std::uint64_t compositionId,
                                 std::uint64_t revision, std::uint64_t candidateId) noexcept {
    try {
        const std::uint64_t deadline =
            fcitx5_windows_common_deadline_after_milliseconds(kInputDeadlineMilliseconds);
        if (!connect(deadline) || !handshake(deadline) ||
            fcitx5_windows_common_accept_candidate_select_request(
                engineEpoch_, expectedEngineEpoch, targetProcessId, contextId, compositionId,
                revision, candidateId) == 0) {
            return false;
        }
        const auto requestId = fcitx5_windows_common_next_pipe_client_request_id();
        const FcitxCandidateSelectRequestC request{
            FcitxMetadataC{requestId, 0, engineEpoch_, sessionId_, contextId, compositionId,
                           revision},
            targetProcessId,
            candidateId};
        std::vector<std::uint8_t> requestBytes;
        if (!encodeMessage(request, requestBytes,
                           fcitx5_protocol_core_encode_candidate_select_request))
            return false;
        std::vector<std::uint8_t> responseBytes;
        if (!transact(requestBytes, responseBytes, deadline)) return false;
        FcitxCandidateSelectResponseC response{};
        std::vector<std::uint8_t> strings;
        if (!decodeMessage(responseBytes, kCandidateSelectResponseType, response, strings,
                           fcitx5_protocol_core_decode_candidate_select_response) ||
            fcitx5_windows_common_accept_candidate_select_response(
                response.metadata.responseTo, response.metadata.engineEpoch,
                response.metadata.sessionId, response.metadata.contextId,
                response.metadata.revision, response.status, requestId, engineEpoch_, sessionId_,
                contextId, revision) == 0) {
            disconnect();
            return false;
        }
        return true;
    } catch (...) {
        disconnect();
        return false;
    }
}

bool PipeClient::pollState(std::uint64_t contextId, KeyResult& result) noexcept {
    result = {};
    try {
        const std::uint64_t deadline =
            fcitx5_windows_common_deadline_after_milliseconds(kInputDeadlineMilliseconds);
        const auto found = contexts_.find(contextId);
        if (found == contexts_.end() || !connect(deadline) || !handshake(deadline)) return false;
        auto& contextState = found->second;
        const auto requestId = fcitx5_windows_common_next_pipe_client_request_id();
        const FcitxStateRequestC request{
            FcitxMetadataC{requestId, 0, engineEpoch_, sessionId_, contextId,
                           contextState.compositionId, contextState.revision}};
        std::vector<std::uint8_t> requestBytes;
        if (!encodeMessage(request, requestBytes, fcitx5_protocol_core_encode_state_request))
            return false;
        std::vector<std::uint8_t> responseBytes;
        if (!transact(requestBytes, responseBytes, deadline)) return false;
        FcitxKeyResponseC response{};
        std::vector<std::uint8_t> strings;
        std::vector<FcitxCandidateRecordC> candidates;
        if (!decodeKeyResponse(responseBytes, response, strings, candidates) ||
            !acceptKeyResponse(response, requestId, contextId, contextState, result)) {
            disconnect();
            return false;
        }
        return true;
    } catch (...) {
        disconnect();
        result = {};
        return false;
    }
}

bool PipeClient::queryEngineStatus(EngineStatusResult& result,
                                   DWORD timeoutMilliseconds) noexcept {
    result = {};
    try {
        const std::uint64_t deadline =
            fcitx5_windows_common_deadline_after_milliseconds(timeoutMilliseconds);
        if (!connect(deadline) || !handshake(deadline)) {
            disconnect();
            return false;
        }
        const auto requestId = fcitx5_windows_common_next_pipe_client_request_id();
        const FcitxEngineStatusRequestC request{
            FcitxMetadataC{requestId, 0, engineEpoch_, sessionId_, 0, 0, 0}};
        std::vector<std::uint8_t> requestBytes;
        if (!encodeMessage(request, requestBytes,
                           fcitx5_protocol_core_encode_engine_status_request))
            return false;
        std::vector<std::uint8_t> responseBytes;
        if (!transact(requestBytes, responseBytes, deadline)) return false;
        FcitxEngineStatusResponseC response{};
        std::vector<std::uint8_t> strings;
        if (!decodeMessage(responseBytes, kEngineStatusResponseType, response, strings,
                           fcitx5_protocol_core_decode_engine_status_response)) {
            disconnect();
            return false;
        }
        const auto scalars = fcitx5_windows_common_apply_engine_status_response_scalars(
            Fcitx5WindowsCommonEngineStatusResponseScalarInput{
                response.metadata.requestId, response.metadata.responseTo,
                response.metadata.engineEpoch, response.metadata.sessionId,
                response.metadata.contextId, response.metadata.compositionId,
                response.metadata.revision, response.status, requestId, engineEpoch_,
                sessionId_});
        if (scalars.status == 0) {
            disconnect();
            return false;
        }
        result.requestId = scalars.requestId;
        result.responseTo = scalars.responseTo;
        result.engineEpoch = scalars.engineEpoch;
        result.sessionId = scalars.sessionId;
        result.contextId = scalars.contextId;
        result.compositionId = scalars.compositionId;
        result.revision = scalars.revision;
        result.status = scalars.responseStatus;
        result.currentInputMethodId.assign(byteView(response.currentInputMethodId));
        result.currentInputMethodName.assign(byteView(response.currentInputMethodName));
        result.currentInputMethodNativeName.assign(byteView(response.currentInputMethodNativeName));
        result.currentInputMethodShortLabel.assign(byteView(response.currentInputMethodShortLabel));
        return true;
    } catch (...) {
        disconnect();
        result = {};
        return false;
    }
}

} // namespace fcitx::windows::ipc
