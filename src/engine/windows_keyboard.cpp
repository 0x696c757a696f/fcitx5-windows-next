// SPDX-License-Identifier: LGPL-2.1-or-later

#include <fcitx/addonfactory.h>
#include <fcitx/addonmanager.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/inputmethodentry.h>

namespace fcitx::windows::engine {

// Fcitx groups intentionally use a keyboard input method as their first item.
// On Windows the TSF host already owns layout handling, so this engine must not
// consume anything: an unaccepted event is returned to the application by TSF.
class WindowsKeyboardEngine final : public InputMethodEngineV3 {
  public:
    void keyEvent(const InputMethodEntry&, KeyEvent&) override {}

    std::vector<InputMethodEntry> listInputMethods() override {
        std::vector<InputMethodEntry> result;
        InputMethodEntry entry("keyboard-us", "Keyboard", "en", "windowskeyboard");
        entry.setLabel("En").setIcon("input-keyboard");
        result.emplace_back(std::move(entry));
        return result;
    }
};

class WindowsKeyboardEngineFactory final : public AddonFactory {
  public:
    AddonInstance* create(AddonManager*) override { return new WindowsKeyboardEngine(); }
};

} // namespace fcitx::windows::engine

FCITX_ADDON_FACTORY_V2(windowskeyboard,
                       fcitx::windows::engine::WindowsKeyboardEngineFactory);
