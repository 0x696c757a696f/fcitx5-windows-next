#include "presentation_publisher.h"

#include "peer_verification.h"

#include <utility>

namespace fcitx::windows::engine {

PresentationPublisher::PresentationPublisher(std::wstring pipeName,
                                             std::wstring uiExecutable)
    : pipeName_(std::move(pipeName)), uiExecutable_(std::move(uiExecutable)),
      thread_([this] { run(); }) {}

PresentationPublisher::~PresentationPublisher() {
    {
        std::lock_guard lock(mutex_);
        stopping_ = true;
    }
    condition_.notify_one();
    if (thread_.joinable()) thread_.join();
    disconnect();
}

void PresentationPublisher::publish(protocol::KeyResponse response) {
    {
        std::lock_guard lock(mutex_);
        latest_ = std::move(response);
    }
    condition_.notify_one();
}

void PresentationPublisher::run() {
    for (;;) {
        std::optional<protocol::KeyResponse> response;
        {
            std::unique_lock lock(mutex_);
            condition_.wait(lock, [this] { return stopping_ || latest_.has_value(); });
            if (stopping_) return;
            response = std::move(latest_);
            latest_.reset();
        }
        if (!send(*response)) disconnect();
    }
}

bool PresentationPublisher::connect() {
    if (pipe_ != INVALID_HANDLE_VALUE) return true;
    if (!WaitNamedPipeW(pipeName_.c_str(), 5)) return false;
    pipe_ = CreateFileW(pipeName_.c_str(), GENERIC_WRITE, 0, nullptr, OPEN_EXISTING,
                        FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OVERLAPPED, nullptr);
    if (pipe_ == INVALID_HANDLE_VALUE) return false;
    platform::RuntimeIdentity identity;
    if (!platform::queryCurrentIdentity(identity) ||
        !ipc::verifyPipeServer(pipe_, identity,
                               ipc::PeerPolicy::exact(uiExecutable_))) {
        disconnect();
        return false;
    }
    return true;
}

bool PresentationPublisher::send(const protocol::KeyResponse& response) {
    if (!connect()) return false;
    const auto bytes = protocol::encode(response);
    if (bytes.empty()) return false;
    std::size_t offset = 0;
    while (offset < bytes.size()) {
        HANDLE event = CreateEventW(nullptr, TRUE, FALSE, nullptr);
        if (!event) return false;
        OVERLAPPED operation{};
        operation.hEvent = event;
        DWORD written = 0;
        const DWORD request = static_cast<DWORD>(bytes.size() - offset);
        bool success = WriteFile(pipe_, bytes.data() + offset, request, &written,
                                 &operation) != FALSE;
        if (!success && GetLastError() == ERROR_IO_PENDING) {
            if (WaitForSingleObject(event, 25) == WAIT_OBJECT_0) {
                success = GetOverlappedResult(pipe_, &operation, &written, FALSE) != FALSE;
            } else {
                CancelIoEx(pipe_, &operation);
                (void)GetOverlappedResult(pipe_, &operation, &written, TRUE);
            }
        }
        CloseHandle(event);
        if (!success || written == 0)
            return false;
        offset += written;
    }
    return true;
}

void PresentationPublisher::disconnect() noexcept {
    if (pipe_ != INVALID_HANDLE_VALUE) CloseHandle(std::exchange(pipe_, INVALID_HANDLE_VALUE));
}

} // namespace fcitx::windows::engine
