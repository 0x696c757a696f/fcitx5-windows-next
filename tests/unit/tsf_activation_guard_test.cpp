#include "activation_guard.h"

#include <Windows.h>

#include <chrono>
#include <filesystem>
#include <fstream>
#include <iostream>

namespace {

namespace fs = std::filesystem;
using namespace std::chrono_literals;

[[nodiscard]] fs::path temporaryRoot() {
    std::wstring buffer(32768, L'\0');
    const DWORD length = GetTempPathW(static_cast<DWORD>(buffer.size()), buffer.data());
    if (length == 0 || length >= buffer.size()) return {};
    buffer.resize(length);
    return fs::path(buffer) /
           (L"fcitx5-tsf-activation-guard-" + std::to_wstring(GetCurrentProcessId()));
}

[[nodiscard]] bool require(bool condition, const char* message) {
    if (condition) return true;
    std::cerr << message << '\n';
    return false;
}

} // namespace

int main() {
    const auto root = temporaryRoot();
    if (root.empty()) {
        std::cerr << "temporary root unavailable\n";
        return 1;
    }
    std::error_code ignored;
    fs::remove_all(root, ignored);
    fs::create_directories(root, ignored);

    bool ok = true;
    ok = require(!fcitx::windows::tsf::activationGuardStatus(root).disabled,
                 "fresh guard should be enabled") &&
         ok;
    {
        auto attempt = fcitx::windows::tsf::ActivationAttempt::begin(root, 60s);
        ok = require(!attempt.failOpen(), "fresh activation attempt should not fail open") && ok;
        attempt.finish();
    }
    ok = require(!fcitx::windows::tsf::activationGuardStatus(root).disabled,
                 "finished activation should not disable TSF") &&
         ok;
    ok = require(fcitx::windows::tsf::disableActivationGuard(root, "manual_recovery"),
                 "manual disable should write marker") &&
         ok;
    auto disabled = fcitx::windows::tsf::activationGuardStatus(root);
    ok = require(disabled.disabled && disabled.reason == "manual_recovery",
                 "manual disable status should include reason") &&
         ok;
    {
        auto attempt = fcitx::windows::tsf::ActivationAttempt::begin(root, 60s);
        ok = require(attempt.failOpen() && attempt.reason() == "manual_recovery",
                     "disabled marker should force fail-open activation") &&
             ok;
    }
    ok = require(fcitx::windows::tsf::clearActivationGuard(root),
                 "clear should remove guard state") &&
         ok;
    ok = require(!fcitx::windows::tsf::activationGuardStatus(root).disabled,
                 "clear should re-enable guard") &&
         ok;

    const auto stale = root / L"recovery" / L"tsf-activation-attempt.stale.v1";
    fs::create_directories(stale.parent_path(), ignored);
    {
        std::ofstream file(stale, std::ios::binary);
        file << "format_version=1\nstate=activating\n";
    }
    fs::last_write_time(stale, fs::file_time_type::clock::now() - 120s, ignored);
    {
        auto attempt = fcitx::windows::tsf::ActivationAttempt::begin(root, 1s);
        ok = require(attempt.failOpen() &&
                         attempt.reason() == "previous_activation_did_not_finish",
                     "stale activation should force fail-open marker") &&
             ok;
    }
    ok = require(fcitx::windows::tsf::activationGuardStatus(root).disabled,
                 "stale activation should persist disabled marker") &&
         ok;

    fs::remove_all(root, ignored);
    return ok ? 0 : 1;
}
