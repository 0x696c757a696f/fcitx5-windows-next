#include "pipe_client.h"

#include "launcher_client.h"
#include "protocol.h"

#include <algorithm>
#include <array>
#include <limits>
#include <span>
#include <vector>

namespace fcitx::windows::ipc {
namespace {

extern "C" std::uint8_t fcitx5_windows_common_pipe_transfer(
    void* pipe,
    std::uint8_t write,
    std::uint8_t* data,
    std::size_t size,
    std::uint64_t deadline);

DWORD remainingMilliseconds(std::uint64_t deadline) noexcept {
    const auto now = GetTickCount64();
    if (now >= deadline) {
        return 0;
    }
    const auto remaining = deadline - now;
    return static_cast<DWORD>((std::min)(remaining, static_cast<std::uint64_t>(MAXDWORD - 1)));
}

bool utf8ToWide(std::string_view input, std::wstring& output) {
    output.clear();
    if (input.empty()) {
        return true;
    }
    const int size = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, input.data(),
                                         static_cast<int>(input.size()), nullptr, 0);
    if (size <= 0) {
        return false;
    }
    output.resize(static_cast<std::size_t>(size));
    return MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, input.data(),
                               static_cast<int>(input.size()), output.data(), size) == size;
}

bool utf8OffsetToWide(std::string_view input, std::uint32_t offset,
                      std::uint32_t& output) {
    if (offset > input.size()) return false;
    if (offset == 0) {
        output = 0;
        return true;
    }
    const int size = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, input.data(),
                                         static_cast<int>(offset), nullptr, 0);
    if (size <= 0) return false;
    output = static_cast<std::uint32_t>(size);
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
    if (remainingMilliseconds(deadline) == 0 || pipeName_.empty() ||
        !identity_.mayUseUserEngine()) {
        return false;
    }
    for (;;) {
        pipe_ = CreateFileW(pipeName_.c_str(), GENERIC_READ | GENERIC_WRITE, 0, nullptr,
                            OPEN_EXISTING, FILE_FLAG_OVERLAPPED, nullptr);
        if (pipe_ != INVALID_HANDLE_VALUE) {
            break;
        }
        const DWORD error = GetLastError();
        const DWORD wait = remainingMilliseconds(deadline);
        if (error != ERROR_PIPE_BUSY || wait == 0 ||
            !WaitNamedPipeW(pipeName_.c_str(), wait)) {
            SetLastError(error);
            return false;
        }
    }
    if (!verifyPipeServer(pipe_, identity_, peerPolicy_)) {
        disconnect();
        return false;
    }
    return true;
}

bool PipeClient::transfer(bool write, void* data, std::size_t size,
                          std::uint64_t deadline) noexcept {
    return fcitx5_windows_common_pipe_transfer(
               pipe_, write ? 1 : 0, static_cast<std::uint8_t*>(data), size, deadline) != 0;
}

bool PipeClient::transact(const std::vector<std::uint8_t>& request,
                          std::vector<std::uint8_t>& response,
                          std::uint64_t deadline) noexcept {
    if (request.empty() || request.size() > protocol::kMaxFrameSize ||
        !transfer(true, const_cast<std::uint8_t*>(request.data()), request.size(), deadline)) {
        disconnect();
        return false;
    }
    std::array<std::uint8_t, protocol::kHeaderSize> header{};
    if (!transfer(false, header.data(), header.size(), deadline)) {
        disconnect();
        return false;
    }
    protocol::MessageType type{};
    std::uint32_t bodySize = 0;
    protocol::Metadata metadata;
    if (!protocol::decodeHeader(header, type, bodySize, metadata)) {
        disconnect();
        return false;
    }
    try {
        response.assign(header.begin(), header.end());
        response.resize(protocol::kHeaderSize + bodySize);
    } catch (...) {
        disconnect();
        return false;
    }
    if (bodySize > 0 &&
        !transfer(false, response.data() + protocol::kHeaderSize, bodySize, deadline)) {
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
            static_cast<std::uint32_t>(sizeof(void*) * 8), GetCurrentProcessId()};
        std::vector<std::uint8_t> responseBytes;
        if (!transact(protocol::encode(request), responseBytes, deadline)) {
            return false;
        }
        protocol::FrameView frame;
        protocol::HelloResponse response;
        if (!protocol::decodeFrame(responseBytes, frame) || !protocol::decode(frame, response) ||
            response.metadata.responseTo != requestId ||
            response.metadata.sessionId != sessionId_ || response.status != protocol::Status::ok) {
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
    if (response.metadata.responseTo != requestId ||
        response.metadata.engineEpoch != engineEpoch_ ||
        response.metadata.sessionId != sessionId_ ||
        response.metadata.contextId != contextId ||
        response.metadata.revision <= contextState.revision ||
        response.status != protocol::Status::ok ||
        !utf8ToWide(response.commitUtf8, result.commit) ||
        !utf8ToWide(response.preeditUtf8, result.preedit) ||
        !utf8OffsetToWide(response.preeditUtf8, response.preeditCaretUtf8,
                          result.preeditCaretUtf16)) {
        return false;
    }
    contextState.compositionId = response.metadata.compositionId;
    contextState.revision = response.metadata.revision;
    result.engineEpoch = response.metadata.engineEpoch;
    result.compositionId = response.metadata.compositionId;
    result.revision = response.metadata.revision;
    result.handled = response.handled;
    result.selectedCandidate = response.selectedCandidate;
    result.candidatePage = response.candidatePage;
    result.candidateTotal = response.candidateTotal;
    result.candidateVisibility = response.candidateVisibility;
    result.deleteSurroundingText = response.deleteSurroundingText;
    result.deleteSurroundingOffset = response.deleteSurroundingOffset;
    result.deleteSurroundingSize = response.deleteSurroundingSize;
    result.forwardKey = response.forwardKey;
    result.forwardKeySym = response.forwardKeySym;
    result.forwardKeyStates = response.forwardKeyStates;
    result.forwardKeyCode = response.forwardKeyCode;
    result.forwardKeyRelease = response.forwardKeyRelease;
    result.caret = response.caret;
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
        const std::uint64_t deadline =
            GetTickCount64() + (newContext ? kContextStartDeadlineMilliseconds
                                           : kInputDeadlineMilliseconds);
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
        const std::uint64_t deadline = GetTickCount64() + kInputDeadlineMilliseconds;
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
            response.metadata.responseTo != requestId ||
            response.metadata.engineEpoch != engineEpoch_ ||
            response.metadata.sessionId != sessionId_ ||
            response.metadata.contextId != contextId ||
            response.metadata.revision <= revision || response.status != protocol::Status::ok) {
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
        const std::uint64_t deadline = GetTickCount64() + kInputDeadlineMilliseconds;
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
        const std::uint64_t deadline = GetTickCount64() + timeoutMilliseconds;
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
            response.metadata.responseTo != requestId ||
            response.metadata.engineEpoch != engineEpoch_ ||
            response.metadata.sessionId != sessionId_ ||
            response.status != protocol::Status::ok) {
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
