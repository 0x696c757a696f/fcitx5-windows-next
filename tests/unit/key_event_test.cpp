#include "key_event.h"

#include <fcitx-utils/key.h>
#include <Windows.h>

#include <iostream>

namespace {

bool expect(bool condition, const char* message) {
    if (!condition) {
        std::cerr << message << '\n';
    }
    return condition;
}

fcitx::windows::protocol::KeyRequest request(std::uint32_t virtualKey, std::uint32_t flags,
                                             std::uint32_t scanCode, const char* logicalText = "") {
    fcitx::windows::protocol::KeyRequest result;
    result.metadata = {1, 0, 1, 1, 1, 0, 0};
    result.virtualKey = virtualKey;
    result.keyFlags = flags;
    result.scanCode = scanCode;
    result.logicalTextUtf8 = logicalText;
    result.keyboardLayout = 0x04090409ULL;
    return result;
}

} // namespace

int main() {
    using namespace fcitx;
    using namespace fcitx::windows;

    const auto letter = engine::keyFromRequest(request('N', 0, 0x31, "n"));
    if (!expect(letter.sym() == FcitxKey_n && letter.code() == 0x31,
                "ordinary logical letter did not reach Fcitx key")) {
        return 1;
    }

    const auto punctuation = engine::keyFromRequest(request(VK_OEM_1, 0, 0x27, ";"));
    if (!expect(punctuation.sym() == FcitxKey_semicolon && punctuation.code() == 0x27,
                "punctuation/quickphrase key did not use logical text")) {
        return 1;
    }

    const auto altGr = engine::keyFromRequest(
        request('Q', protocol::kKeyFlagControl | protocol::kKeyFlagAlt | protocol::kKeyFlagAltGr,
                0x10, "@"));
    if (!expect(altGr.sym() == FcitxKey_at && !(altGr.states() & KeyState::Ctrl) &&
                    !(altGr.states() & KeyState::Alt),
                "AltGr printable key degraded into Ctrl+Alt shortcut state")) {
        return 1;
    }

    const auto nonUs = engine::keyFromRequest(request(VK_OEM_3, 0, 0x29, "\xc3\xb1"));
    if (!expect(nonUs.sym() == Key::keySymFromUnicode(0x00f1),
                "non-US logical character did not become a Unicode keysym")) {
        return 1;
    }

    auto extended = request(VK_RIGHT, 0, 0x4d);
    extended.extendedKey = true;
    const auto right = engine::keyFromRequest(extended);
    if (!expect(right.sym() == FcitxKey_Right && right.code() == 0x4d,
                "extended/scancode key identity was not preserved")) {
        return 1;
    }

    const auto release = engine::keyFromRequest(
        request(VK_SHIFT, protocol::kKeyFlagRelease | protocol::kKeyFlagControl, 0x2a));
    if (!expect(release.sym() == FcitxKey_Shift_L && (release.states() & KeyState::Ctrl) &&
                    !(release.states() & KeyState::Alt),
                "key-up modifier event lost its normalized key identity")) {
        return 1;
    }

    const auto dead =
        engine::keyFromRequest(request(VK_OEM_6, protocol::kKeyFlagDeadKey, 0x1b, "^"));
    if (!expect(dead.sym() == FcitxKey_asciicircum,
                "dead-key logical text was not represented as a keysym")) {
        return 1;
    }

    const auto chttransHotkey = engine::keyFromRequest(request(
        'F', protocol::kKeyFlagControl | protocol::kKeyFlagShift, 0x21));
    const fcitx::KeyList chttransHotkeys{fcitx::Key("Control+Shift+F")};
    if (!expect(chttransHotkey.checkKeyList(chttransHotkeys),
                "Ctrl+Shift+F did not match the Fcitx chttrans action hotkey")) {
        return 1;
    }

    return 0;
}
