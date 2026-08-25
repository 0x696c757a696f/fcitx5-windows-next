#include "key_event.h"

#include <Windows.h>

#include <string_view>

namespace fcitx::windows::engine {
namespace {

std::uint32_t firstUtf8CodePoint(std::string_view text) noexcept {
    if (text.empty())
        return 0;
    const auto byte0 = static_cast<unsigned char>(text[0]);
    if (byte0 < 0x80U)
        return byte0;
    if ((byte0 & 0xe0U) == 0xc0U && text.size() >= 2) {
        const auto byte1 = static_cast<unsigned char>(text[1]);
        if ((byte1 & 0xc0U) == 0x80U)
            return ((byte0 & 0x1fU) << 6U) | (byte1 & 0x3fU);
    }
    if ((byte0 & 0xf0U) == 0xe0U && text.size() >= 3) {
        const auto byte1 = static_cast<unsigned char>(text[1]);
        const auto byte2 = static_cast<unsigned char>(text[2]);
        if ((byte1 & 0xc0U) == 0x80U && (byte2 & 0xc0U) == 0x80U)
            return ((byte0 & 0x0fU) << 12U) | ((byte1 & 0x3fU) << 6U) | (byte2 & 0x3fU);
    }
    if ((byte0 & 0xf8U) == 0xf0U && text.size() >= 4) {
        const auto byte1 = static_cast<unsigned char>(text[1]);
        const auto byte2 = static_cast<unsigned char>(text[2]);
        const auto byte3 = static_cast<unsigned char>(text[3]);
        if ((byte1 & 0xc0U) == 0x80U && (byte2 & 0xc0U) == 0x80U && (byte3 & 0xc0U) == 0x80U)
            return ((byte0 & 0x07U) << 18U) | ((byte1 & 0x3fU) << 12U) | ((byte2 & 0x3fU) << 6U) |
                   (byte3 & 0x3fU);
    }
    return 0;
}

KeyStates statesFromRequest(const protocol::KeyRequest& request) noexcept {
    KeyStates states;
    const bool altGr = (request.keyFlags & protocol::kKeyFlagAltGr) != 0;
    const bool printableAltGr = altGr && !request.logicalTextUtf8.empty();
    if ((request.keyFlags & protocol::kKeyFlagShift) != 0)
        states |= KeyState::Shift;
    if (!printableAltGr && (request.keyFlags & protocol::kKeyFlagControl) != 0)
        states |= KeyState::Ctrl;
    if (!printableAltGr && (request.keyFlags & protocol::kKeyFlagAlt) != 0)
        states |= KeyState::Alt;
    if ((request.keyFlags & protocol::kKeyFlagSuper) != 0)
        states |= KeyState::Super;
    return states;
}

KeySym logicalKeySym(std::string_view text) noexcept {
    const std::uint32_t codePoint = firstUtf8CodePoint(text);
    return codePoint == 0 ? FcitxKey_None : Key::keySymFromUnicode(codePoint);
}

} // namespace

Key keyFromRequest(const protocol::KeyRequest& request) {
    const KeyStates states = statesFromRequest(request);
    if (const KeySym sym = logicalKeySym(request.logicalTextUtf8);
        sym != FcitxKey_None && sym != FcitxKey_VoidSymbol) {
        return Key(sym, states, static_cast<int>(request.scanCode));
    }

    const auto vk = request.virtualKey;
    switch (vk) {
    case VK_BACK:
        return Key(FcitxKey_BackSpace, states, static_cast<int>(request.scanCode));
    case VK_RETURN:
        return Key(FcitxKey_Return, states, static_cast<int>(request.scanCode));
    case VK_SPACE:
        return Key(FcitxKey_space, states, static_cast<int>(request.scanCode));
    case VK_ESCAPE:
        return Key(FcitxKey_Escape, states, static_cast<int>(request.scanCode));
    case VK_SHIFT:
        return Key(FcitxKey_Shift_L, states, static_cast<int>(request.scanCode));
    case VK_CONTROL:
        return Key(FcitxKey_Control_L, states, static_cast<int>(request.scanCode));
    case VK_MENU:
        return Key(FcitxKey_Alt_L, states, static_cast<int>(request.scanCode));
    case VK_LEFT:
        return Key(FcitxKey_Left, states, static_cast<int>(request.scanCode));
    case VK_RIGHT:
        return Key(FcitxKey_Right, states, static_cast<int>(request.scanCode));
    case VK_UP:
        return Key(FcitxKey_Up, states, static_cast<int>(request.scanCode));
    case VK_DOWN:
        return Key(FcitxKey_Down, states, static_cast<int>(request.scanCode));
    case VK_PRIOR:
        return Key(FcitxKey_Page_Up, states, static_cast<int>(request.scanCode));
    case VK_NEXT:
        return Key(FcitxKey_Page_Down, states, static_cast<int>(request.scanCode));
    case VK_HOME:
        return Key(FcitxKey_Home, states, static_cast<int>(request.scanCode));
    case VK_END:
        return Key(FcitxKey_End, states, static_cast<int>(request.scanCode));
    case VK_OEM_PLUS:
        return Key((request.keyFlags & protocol::kKeyFlagShift) != 0 ? FcitxKey_plus
                                                                     : FcitxKey_equal,
                   states, static_cast<int>(request.scanCode));
    case VK_OEM_MINUS:
        return Key((request.keyFlags & protocol::kKeyFlagShift) != 0 ? FcitxKey_underscore
                                                                     : FcitxKey_minus,
                   states, static_cast<int>(request.scanCode));
    case VK_OEM_COMMA:
        return Key((request.keyFlags & protocol::kKeyFlagShift) != 0 ? FcitxKey_less
                                                                     : FcitxKey_comma,
                   states, static_cast<int>(request.scanCode));
    case VK_OEM_PERIOD:
        return Key((request.keyFlags & protocol::kKeyFlagShift) != 0 ? FcitxKey_greater
                                                                     : FcitxKey_period,
                   states, static_cast<int>(request.scanCode));
    case VK_OEM_1:
        return Key((request.keyFlags & protocol::kKeyFlagShift) != 0 ? FcitxKey_colon
                                                                     : FcitxKey_semicolon,
                   states, static_cast<int>(request.scanCode));
    case VK_OEM_7:
        return Key((request.keyFlags & protocol::kKeyFlagShift) != 0 ? FcitxKey_quotedbl
                                                                     : FcitxKey_apostrophe,
                   states, static_cast<int>(request.scanCode));
    case VK_OEM_4:
        return Key((request.keyFlags & protocol::kKeyFlagShift) != 0 ? FcitxKey_braceleft
                                                                     : FcitxKey_bracketleft,
                   states, static_cast<int>(request.scanCode));
    case VK_OEM_6:
        return Key((request.keyFlags & protocol::kKeyFlagShift) != 0 ? FcitxKey_braceright
                                                                     : FcitxKey_bracketright,
                   states, static_cast<int>(request.scanCode));
    default:
        break;
    }
    if (vk >= 'A' && vk <= 'Z') {
        const bool shifted = (request.keyFlags & protocol::kKeyFlagShift) != 0;
        return Key(static_cast<KeySym>((shifted ? FcitxKey_A : FcitxKey_a) + (vk - 'A')),
                   states, static_cast<int>(request.scanCode));
    }
    if (vk >= '0' && vk <= '9') {
        return Key(static_cast<KeySym>(FcitxKey_0 + (vk - '0')), states,
                   static_cast<int>(request.scanCode));
    }
    return Key::fromKeyCode(static_cast<int>(request.scanCode != 0 ? request.scanCode : vk),
                            states);
}

} // namespace fcitx::windows::engine
