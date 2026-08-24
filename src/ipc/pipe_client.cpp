#include "pipe_client.h"

#include "launcher_client.h"
#include "protocol.h"

#include <vector>

namespace fcitx::windows::ipc {
namespace {

struct Fcitx5WindowsCommonPipeTransact {
    std::uint8_t status;
    std::size_t responseLen;
};
extern "C" Fcitx5WindowsCommonPipeTransact fcitx5_windows_common_pipe_transact(
    void* pipe,
    const std::uint8_t* request,
    std::size_t request_len,
    std::uint8_t* response_output,
    std::size_t response_capacity,
    std::uint64_t deadline);
extern "C" std::uint64_t fcitx5_windows_common_deadline_after_milliseconds(
    std::uint32_t milliseconds);
extern "C" std::uint32_t fcitx5_windows_common_current_process_id();
extern "C" void* fcitx5_windows_common_open_pipe_client_utf16(
    const std::uint16_t* pipe_name,
    std::size_t pipe_name_len,
    std::uint64_t deadline,
    std::uint8_t wait_when_busy);
struct Fcitx5WindowsCommonUtf8ToWide {
    std::uint8_t status;
    std::size_t utf16Len;
};
struct Fcitx5WindowsCommonUtf8OffsetToWide {
    std::uint8_t status;
    std::uint32_t utf16Offset;
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
extern "C" Fcitx5WindowsCommonUtf8ToWide fcitx5_windows_common_utf8_to_wide_utf16(
    const std::uint8_t* input,
    std::size_t input_len,
    std::uint16_t* output,
    std::size_t capacity);
extern "C" Fcitx5WindowsCommonUtf8OffsetToWide
fcitx5_windows_common_utf8_offset_to_wide(const std::uint8_t* input,
                                          std::size_t input_len,
                                          std::uint32_t offset);
extern "C" Fcitx5WindowsCommonKeyResponseScalars
fcitx5_windows_common_apply_key_response_scalars(
    Fcitx5WindowsCommonKeyResponseScalarInput input);
extern "C" std::uint8_t fcitx5_windows_common_accept_hello_response(
    std::uint64_t response_to,
    std::uint32_t session_id,
    std::uint32_t status,
    std::uint64_t expected_request_id,
    std::uint32_t expected_session_id);
extern "C" std::uint8_t fcitx5_windows_common_accept_candidate_select_response(
    std::uint64_t response_to,
    std::uint64_t engine_epoch,
    std::uint32_t session_id,
    std::uint64_t context_id,
    std::uint64_t revision,
    std::uint32_t status,
    std::uint64_t expected_request_id,
    std::uint64_t expected_engine_epoch,
    std::uint32_t expected_session_id,
    std::uint64_t expected_context_id,
    std::uint64_t previous_revision);
extern "C" std::uint8_t fcitx5_windows_common_accept_engine_status_response(
    std::uint64_t response_to,
    std::uint64_t engine_epoch,
    std::uint32_t session_id,
    std::uint32_t status,
    std::uint64_t expected_request_id,
    std::uint64_t expected_engine_epoch,
    std::uint32_t expected_session_id);

const std::uint16_t* wideData(std::wstring_view value) noexcept {
    static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
    return reinterpret_cast<const std::uint16_t*>(value.data());
}

const std::uint8_t* byteData(std::string_view value) noexcept {
    return value.empty() ? nullptr : reinterpret_cast<const std::uint8_t*>(value.data());
}

std::uint8_t flagByte(bool value) noexcept { return value ? 1 : 0; }

bool utf8ToWide(std::string_view input, std::wstring& output) {
    output.clear();
    const auto query =
        fcitx5_windows_common_utf8_to_wide_utf16(byteData(input), input.size(), nullptr, 0);
    if (query.status == 0) {
        return false;
    }
    output.assign(query.utf16Len, L'\0');
    const auto filled = fcitx5_windows_common_utf8_to_wide_utf16(
        byteData(input), input.size(),
        output.empty() ? nullptr : reinterpret_cast<std::uint16_t*>(output.data()),
        output.size());
    if (filled.status == 0 || filled.utf16Len != output.size()) {
        output.clear();
        return false;
    }
    return true;
}

bool utf8OffsetToWide(std::string_view input, std::uint32_t offset,
                      std::uint32_t& output) {
    const auto converted =
        fcitx5_windows_common_utf8_offset_to_wide(byteData(input), input.size(), offset);
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
    if (platform::queryCurrentIdentity(identity_)) {
        sessionId_ = identity_.sessionId;
    }
}

PipeClient::~PipeClient() { disconnect(); }

void PipeClient::disconnect() noexcept {
    if (pipe_ != INVALID_HANDLE_VALUE) {
        CancelIoEx(pipe_, nullptr);
        CloseHandle(pipe_);
        pipe_ = INVALID_HANDLE_VALUE;
    }
    handshakeComplete_ = false;
    engineEpoch_ = 0;
    contexts_.clear();
}

bool PipeClient::connect(std::uint64_t deadline) noexcept {
    if (pipe_ != INVALID_HANDLE_VALUE) {
        return true;
    }
    if (pipeName_.empty() || !identity_.mayUseUserEngine()) {
        return false;
    }
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
    if (request.empty() || request.size() > protocol::kMaxFrameSize) {
        disconnect();
        return false;
    }
    try {
        response.assign(protocol::kMaxFrameSize, 0);
    } catch (...) {
        disconnect();
        return false;
    }
    const auto transferred = fcitx5_windows_common_pipe_transact(
        pipe_, request.data(), request.size(), response.data(), response.size(), deadline);
    if (transferred.status == 0 || transferred.responseLen < protocol::kHeaderSize ||
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
        if (handshakeComplete_) {
            return true;
        }
        if (sessionId_ == 0) return false;
        const auto requestId = nextRequestId_.fetch_add(1, std::memory_order_relaxed);
        protocol::HelloRequest request{
            protocol::Metadata{requestId, 0, 0, sessionId_, 0, 0, 0},
            static_cast<std::uint32_t>(sizeof(void*) * 8),
            fcitx5_windows_common_current_process_id()};
        std::vector<std::uint8_t> responseBytes;
        if (!transact(protocol::encode(request), responseBytes, deadline)) {
            return false;
        }
        protocol::FrameView frame;
        protocol::HelloResponse response;
        if (!protocol::decodeFrame(responseBytes, frame) || !protocol::decode(frame, response) ||
            fcitx5_windows_common_accept_hello_response(
                response.metadata.responseTo, response.metadata.sessionId,
                static_cast<std::uint32_t>(response.status), requestId, sessionId_) == 0) {
            disconnect();
            return false;
        }
        engineEpoch_ = response.metadata.engineEpoch;
        handshakeComplete_ = true;
        return true;
    } catch (...) {
        disconnect();
        return false;
    }
}

bool PipeClient::acceptKeyResponse(const protocol::KeyResponse& response,
                                   std::uint64_t requestId,
                                   std::uint64_t contextId,
                                   ContextState& contextState,
                                   KeyResult& result) noexcept {
    const Fcitx5WindowsCommonKeyResponseScalarInput scalarInput{
        response.metadata.responseTo,
        response.metadata.engineEpoch,
        response.metadata.sessionId,
        response.metadata.contextId,
        response.metadata.compositionId,
        response.metadata.revision,
        static_cast<std::uint32_t>(response.status),
        requestId,
        engineEpoch_,
        sessionId_,
        contextId,
        contextState.revision,
        flagByte(response.handled),
        response.selectedCandidate,
        response.candidatePage,
        response.candidateTotal,
        response.candidateVisibility,
        flagByte(response.deleteSurroundingText),
        response.deleteSurroundingOffset,
        response.deleteSurroundingSize,
        flagByte(response.forwardKey),
        response.forwardKeySym,
        response.forwardKeyStates,
        response.forwardKeyCode,
        flagByte(response.forwardKeyRelease),
        flagByte(response.caret.valid),
        response.caret.left,
        response.caret.top,
        response.caret.right,
        response.caret.bottom,
        response.caret.dpi};
    const auto scalars = fcitx5_windows_common_apply_key_response_scalars(scalarInput);
    if (scalars.status == 0 || !utf8ToWide(response.commitUtf8, result.commit) ||
        !utf8ToWide(response.preeditUtf8, result.preedit) ||
        !utf8OffsetToWide(response.preeditUtf8, response.preeditCaretUtf8,
                          result.preeditCaretUtf16)) {
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
    result.caret = protocol::CaretRect{scalars.caretValid != 0, scalars.caretLeft,
                                       scalars.caretTop, scalars.caretRight,
                                       scalars.caretBottom, scalars.caretDpi};
    result.candidates.reserve(response.candidates.size());
    for (const auto& source : response.candidates) {
        KeyResult::Candidate candidate;
        candidate.id = source.id;
        if (!utf8ToWide(source.labelUtf8, candidate.label) ||
            !utf8ToWide(source.textUtf8, candidate.text) ||
            !utf8ToWide(source.commentUtf8, candidate.comment)) {
            result = {};
            return false;
        }
        result.candidates.emplace_back(std::move(candidate));
    }
    return true;
}

bool PipeClient::processKey(std::uint64_t contextId, std::uint32_t virtualKey,
                            std::uint32_t keyFlags, KeyResult& result,
                            const protocol::CaretRect& caret, bool popupAllowed,
                            std::uint32_t scanCode, bool extendedKey,
                            std::uint64_t keyboardLayout,
                            std::string_view logicalText,
                            std::string_view inputMethod,
                            bool surroundingTextValid,
                            std::string_view surroundingText,
                            std::uint32_t surroundingCursor,
                            std::uint32_t surroundingAnchor) noexcept {
    result = {};
    try {
        const bool newContext = contexts_.find(contextId) == contexts_.end();
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
        const auto requestId = nextRequestId_.fetch_add(1, std::memory_order_relaxed);
        auto& contextState = contexts_[contextId];
        protocol::KeyRequest request{
            protocol::Metadata{requestId, 0, engineEpoch_, sessionId_, contextId,
                               contextState.compositionId, contextState.revision},
            virtualKey, keyFlags, scanCode, extendedKey, popupAllowed, keyboardLayout,
            std::string(logicalText), std::string(inputMethod),
            surroundingTextValid, std::string(surroundingText),
            surroundingCursor, surroundingAnchor, caret};
        std::vector<std::uint8_t> responseBytes;
        if (!transact(protocol::encode(request), responseBytes, deadline)) {
            return false;
        }
        protocol::FrameView frame;
        protocol::KeyResponse response;
        if (!protocol::decodeFrame(responseBytes, frame) || !protocol::decode(frame, response) ||
            !acceptKeyResponse(response, requestId, contextId, contextState, result)) {
            disconnect();
            return false;
        }
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
                                 std::uint64_t contextId,
                                 std::uint64_t compositionId,
                                 std::uint64_t revision,
                                 std::uint64_t candidateId) noexcept {
    try {
        const std::uint64_t deadline =
            fcitx5_windows_common_deadline_after_milliseconds(kInputDeadlineMilliseconds);
        if (!connect(deadline) || !handshake(deadline) || targetProcessId == 0 ||
            expectedEngineEpoch == 0 || engineEpoch_ != expectedEngineEpoch ||
            contextId == 0 || compositionId == 0 || revision == 0 || candidateId == 0) {
            return false;
        }
        const auto requestId = nextRequestId_.fetch_add(1, std::memory_order_relaxed);
        const protocol::CandidateSelectRequest request{
            protocol::Metadata{requestId, 0, engineEpoch_, sessionId_, contextId,
                               compositionId, revision},
            targetProcessId, candidateId};
        std::vector<std::uint8_t> responseBytes;
        if (!transact(protocol::encode(request), responseBytes, deadline)) return false;
        protocol::FrameView frame;
        protocol::CandidateSelectResponse response;
        if (!protocol::decodeFrame(responseBytes, frame) || !protocol::decode(frame, response) ||
            fcitx5_windows_common_accept_candidate_select_response(
                response.metadata.responseTo, response.metadata.engineEpoch,
                response.metadata.sessionId, response.metadata.contextId,
                response.metadata.revision, static_cast<std::uint32_t>(response.status),
                requestId, engineEpoch_, sessionId_, contextId, revision) == 0) {
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
        const auto requestId = nextRequestId_.fetch_add(1, std::memory_order_relaxed);
        const protocol::StateRequest request{
            protocol::Metadata{requestId, 0, engineEpoch_, sessionId_, contextId,
                               contextState.compositionId, contextState.revision}};
        std::vector<std::uint8_t> responseBytes;
        if (!transact(protocol::encode(request), responseBytes, deadline)) return false;
        protocol::FrameView frame;
        protocol::KeyResponse response;
        if (!protocol::decodeFrame(responseBytes, frame) || !protocol::decode(frame, response) ||
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

bool PipeClient::queryEngineStatus(protocol::EngineStatusResponse& result,
                                   DWORD timeoutMilliseconds) noexcept {
    result = {};
    try {
        const std::uint64_t deadline =
            fcitx5_windows_common_deadline_after_milliseconds(timeoutMilliseconds);
        if (!connect(deadline) || !handshake(deadline)) {
            disconnect();
            return false;
        }
        const auto requestId = nextRequestId_.fetch_add(1, std::memory_order_relaxed);
        const protocol::EngineStatusRequest request{
            protocol::Metadata{requestId, 0, engineEpoch_, sessionId_, 0, 0, 0}};
        std::vector<std::uint8_t> responseBytes;
        if (!transact(protocol::encode(request), responseBytes, deadline)) return false;
        protocol::FrameView frame;
        protocol::EngineStatusResponse response;
        if (!protocol::decodeFrame(responseBytes, frame) ||
            !protocol::decode(frame, response) ||
            fcitx5_windows_common_accept_engine_status_response(
                response.metadata.responseTo, response.metadata.engineEpoch,
                response.metadata.sessionId, static_cast<std::uint32_t>(response.status),
                requestId, engineEpoch_, sessionId_) == 0) {
            disconnect();
            return false;
        }
        result = std::move(response);
        return true;
    } catch (...) {
        disconnect();
        result = {};
        return false;
    }
}

} // namespace fcitx::windows::ipc
