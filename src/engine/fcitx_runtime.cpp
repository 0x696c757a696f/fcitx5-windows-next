#include "fcitx_runtime.h"
#include "config_model.h"
#include <fcitx5_windows/release_identity.h>

#include <fcitx-utils/capabilityflags.h>
#include <fcitx-utils/environ.h>
#include <fcitx-utils/key.h>
#include <fcitx-utils/keysym.h>
#include <fcitx/addonmanager.h>
#include <fcitx/candidatelist.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputmethodentry.h>
#include <fcitx/inputmethodgroup.h>
#include <fcitx/inputmethodmanager.h>
#include <fcitx/inputpanel.h>
#include <fcitx/instance.h>

#include <Windows.h>
#include <ShlObj.h>

#include <algorithm>
#include <filesystem>
#include <fstream>
#include <iterator>
#include <memory>
#include <optional>
#include <sstream>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <utility>

namespace fcitx::windows::engine {
namespace {

struct EngineConfig {
    std::vector<std::string> enabled;
    std::optional<std::string> defaultInputMethod;
    std::optional<std::string> hotkeyToggle;
    std::optional<std::string> hotkeyNext;
};

std::filesystem::path executableDirectory() {
    std::wstring path(32'768, L'\0');
    const DWORD size = GetModuleFileNameW(nullptr, path.data(), static_cast<DWORD>(path.size()));
    if (size == 0 || size >= path.size())
        return {};
    path.resize(size);
    return std::filesystem::path(path).parent_path();
}

std::filesystem::path localDataDirectory() {
    const auto executable = executableDirectory();
    if (!executable.empty() && std::filesystem::exists(executable / L"portable.flag"))
        return executable / L"data";
    if (!executable.empty() &&
        std::filesystem::exists(executable.parent_path() / L"portable.flag"))
        return executable.parent_path() / L"data";
    PWSTR path = nullptr;
    if (FAILED(SHGetKnownFolderPath(FOLDERID_LocalAppData, KF_FLAG_DEFAULT, nullptr, &path)))
        return {};
    std::filesystem::path result(path);
    CoTaskMemFree(path);
    return result / kReleaseIdentity.data_directory;
}

// config.toml is owned by the config module; parse it with the single
// authoritative parser (config_parser.cpp) instead of a second, hand-rolled
// TOML reader that could drift from the config tool's semantics.
EngineConfig readEngineConfig() {
    EngineConfig config;
    const auto data = localDataDirectory();
    std::string text;
    if (!data.empty()) {
        std::ifstream file(data / L"config.toml");
        if (file) {
            std::ostringstream buffer;
            buffer << file.rdbuf();
            text = buffer.str();
        }
    }
    fcitx::windows::config::Config parsed;
    fcitx::windows::config::ParseError error;
    const bool parsedOk = !text.empty() &&
                          fcitx::windows::config::parseConfig(text, parsed, error);
    if (!parsedOk) {
        // Fall back to the canonical defaults owned by the config module,
        // which include [input_methods] and [hotkeys].
        fcitx::windows::config::Config defaults;
        fcitx::windows::config::ParseError defaultError;
        if (fcitx::windows::config::parseConfig(
                fcitx::windows::config::defaultConfigToml(), defaults, defaultError)) {
            parsed = std::move(defaults);
        }
    }
    config.enabled = parsed.enabledInputMethods;
    config.defaultInputMethod = parsed.defaultInputMethod;
    config.hotkeyToggle = parsed.hotkeyToggleInputMethod;
    config.hotkeyNext = parsed.hotkeyNextInputMethod;
    if (config.enabled.empty())
        config.enabled = {"pinyin", "rime", "wbx"};
    if (!config.defaultInputMethod ||
        std::find(config.enabled.begin(), config.enabled.end(),
                  *config.defaultInputMethod) == config.enabled.end())
        config.defaultInputMethod = config.enabled.front();
    if (!config.hotkeyToggle)
        config.hotkeyToggle = "Ctrl+Space";
    if (!config.hotkeyNext)
        config.hotkeyNext = "Ctrl+Shift";
    return config;
}

std::string utf8Path(const std::filesystem::path& path) {
    const auto& native = path.native();
    if (native.empty())
        return {};
    const int size =
        WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, native.data(),
                            static_cast<int>(native.size()), nullptr, 0, nullptr, nullptr);
    if (size <= 0)
        return {};
    std::string output(static_cast<std::size_t>(size), '\0');
    return WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, native.data(),
                               static_cast<int>(native.size()), output.data(), size, nullptr,
                               nullptr) == size
               ? output
               : std::string{};
}

bool setupEnvironment() {
    std::wstring modulePath(32'768, L'\0');
    const DWORD size =
        GetModuleFileNameW(nullptr, modulePath.data(), static_cast<DWORD>(modulePath.size()));
    if (size == 0 || size >= modulePath.size())
        return false;
    modulePath.resize(size);
    const auto root = std::filesystem::path(modulePath).parent_path().parent_path();
    if (!getEnvironment("FCITX_USER_DATA_ROOT") &&
        std::filesystem::exists(root / "portable.flag")) {
        const auto portableData = utf8Path(root / "data");
        if (portableData.empty())
            return false;
        setEnvironment("FCITX_USER_DATA_ROOT", portableData.c_str());
    }
    using SetDefaultDirectories = BOOL(WINAPI*)(DWORD);
    using AddDirectory = DLL_DIRECTORY_COOKIE(WINAPI*)(PCWSTR);
    const HMODULE kernel = GetModuleHandleW(L"kernel32.dll");
    const auto setDefaultDirectories = kernel
                                           ? reinterpret_cast<SetDefaultDirectories>(
                                                 GetProcAddress(kernel, "SetDefaultDllDirectories"))
                                           : nullptr;
    const auto addDirectory =
        kernel ? reinterpret_cast<AddDirectory>(GetProcAddress(kernel, "AddDllDirectory"))
               : nullptr;
    if (!setDefaultDirectories || !addDirectory ||
        !setDefaultDirectories(LOAD_LIBRARY_SEARCH_APPLICATION_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32 |
                               LOAD_LIBRARY_SEARCH_USER_DIRS) ||
        !addDirectory((root / "bin").c_str())) {
        return false;
    }
    const auto addon = utf8Path(root / "lib" / "fcitx5");
    const auto share = utf8Path(root / "share");
    const auto data = utf8Path(root / "share" / "fcitx5");
    const auto models = utf8Path(root / "lib" / "libime");
    if (addon.empty() || share.empty() || data.empty() || models.empty())
        return false;
    setEnvironment("FCITX_ADDON_DIRS", addon.c_str());
    setEnvironment("XDG_DATA_DIRS", share.c_str());
    setEnvironment("FCITX_DATA_DIRS", data.c_str());
    setEnvironment("LIBIME_MODEL_DIRS", models.c_str());
    return true;
}

struct KeyHash {
    std::size_t operator()(const ClientContextKey& key) const noexcept {
        const auto high = static_cast<std::size_t>(key.contextId >> 32U);
        const auto low = static_cast<std::size_t>(key.contextId);
        const auto connectionHigh = static_cast<std::size_t>(key.connectionId >> 32U);
        const auto connectionLow = static_cast<std::size_t>(key.connectionId);
        return (static_cast<std::size_t>(key.processId) * 0x9e3779b1U) ^
               (connectionHigh * 0x85ebca6bU) ^ connectionLow ^ high ^ low;
    }
};

class EngineInputContext final : public InputContext {
  public:
    explicit EngineInputContext(InputContextManager& manager) : InputContext(manager, "tsf") {
        setEnablePreedit(true);
        setCapabilityFlags(CapabilityFlags{CapabilityFlag::Preedit,
                                           CapabilityFlag::FormattedPreedit,
                                           CapabilityFlag::ClientSideInputPanel});
        created();
    }

    ~EngineInputContext() override { destroy(); }

    const char* frontend() const override { return "tsf"; }

    std::string takeCommit() { return std::exchange(commit_, {}); }

  private:
    void commitStringImpl(const std::string& text) override { commit_ += text; }
    void forwardKeyImpl(const ForwardKeyEvent&) override {}
    void deleteSurroundingTextImpl(int, unsigned int) override {}
    void updatePreeditImpl() override {}

    std::string commit_;
};

Key keyFromRequest(const protocol::KeyRequest& request) {
    KeyStates states;
    if ((request.keyFlags & protocol::kKeyFlagShift) != 0)
        states |= KeyState::Shift;
    if ((request.keyFlags & protocol::kKeyFlagControl) != 0)
        states |= KeyState::Ctrl;
    if ((request.keyFlags & protocol::kKeyFlagAlt) != 0)
        states |= KeyState::Alt;
    if ((request.keyFlags & protocol::kKeyFlagSuper) != 0)
        states |= KeyState::Super;
    const auto vk = request.virtualKey;
    switch (vk) {
    case VK_BACK:
        return Key(FcitxKey_BackSpace, states);
    case VK_RETURN:
        return Key(FcitxKey_Return, states);
    case VK_SPACE:
        return Key(FcitxKey_space, states);
    case VK_ESCAPE:
        return Key(FcitxKey_Escape, states);
    case VK_SHIFT:
        // Modifier key events carry the engine hotkeys Ctrl+Shift / Alt+Shift
        // (matched by keySym on the Shift key itself); Fcitx also tracks the
        // modifier release from key-up events.
        return Key(FcitxKey_Shift_L, states);
    case VK_CONTROL:
        return Key(FcitxKey_Control_L, states);
    case VK_MENU:
        return Key(FcitxKey_Alt_L, states);
    case VK_LEFT:
        return Key(FcitxKey_Left, states);
    case VK_RIGHT:
        return Key(FcitxKey_Right, states);
    case VK_UP:
        return Key(FcitxKey_Up, states);
    case VK_DOWN:
        return Key(FcitxKey_Down, states);
    case VK_PRIOR:
        return Key(FcitxKey_Page_Up, states);
    case VK_NEXT:
        return Key(FcitxKey_Page_Down, states);
    case VK_HOME:
        // Scroll-mode row start (fcitx5-macos ScrollConfig rowStart).
        return Key(FcitxKey_Home, states);
    case VK_END:
        // Scroll-mode row end (fcitx5-macos ScrollConfig rowEnd).
        return Key(FcitxKey_End, states);
    case VK_OEM_PLUS:
        // Laptop-friendly next page: '=' without Shift, '+' with Shift.
        return (request.keyFlags & protocol::kKeyFlagShift) != 0
                   ? Key(FcitxKey_plus, states)
                   : Key(FcitxKey_equal, states);
    case VK_OEM_MINUS:
        // Laptop-friendly previous page: '-' without Shift, '_' with Shift.
        return (request.keyFlags & protocol::kKeyFlagShift) != 0
                   ? Key(FcitxKey_underscore, states)
                   : Key(FcitxKey_minus, states);
    case VK_OEM_COMMA:
        // Comma/period page keys: ',' previous page, '.' next page (Shift
        // variants '<' '>' are punctuation passthrough for Fcitx to decide).
        return (request.keyFlags & protocol::kKeyFlagShift) != 0
                   ? Key(FcitxKey_less, states)
                   : Key(FcitxKey_comma, states);
    case VK_OEM_PERIOD:
        return (request.keyFlags & protocol::kKeyFlagShift) != 0
                   ? Key(FcitxKey_greater, states)
                   : Key(FcitxKey_period, states);
    case VK_OEM_1:
        // Semicolon/apostrophe select the 2nd/3rd candidate; shifted variants
        // ':' '"' keep their punctuation meaning.
        return (request.keyFlags & protocol::kKeyFlagShift) != 0
                   ? Key(FcitxKey_colon, states)
                   : Key(FcitxKey_semicolon, states);
    case VK_OEM_7:
        return (request.keyFlags & protocol::kKeyFlagShift) != 0
                   ? Key(FcitxKey_quotedbl, states)
                   : Key(FcitxKey_apostrophe, states);
    case VK_OEM_4:
        // Bracket page keys: '[' previous page, ']' next page.
        return (request.keyFlags & protocol::kKeyFlagShift) != 0
                   ? Key(FcitxKey_braceleft, states)
                   : Key(FcitxKey_bracketleft, states);
    case VK_OEM_6:
        return (request.keyFlags & protocol::kKeyFlagShift) != 0
                   ? Key(FcitxKey_braceright, states)
                   : Key(FcitxKey_bracketright, states);
    default:
        break;
    }
    if (vk >= 'A' && vk <= 'Z') {
        return Key(static_cast<KeySym>(FcitxKey_a + (vk - 'A')), states);
    }
    if (vk >= '0' && vk <= '9') {
        return Key(static_cast<KeySym>(FcitxKey_0 + (vk - '0')), states);
    }
    return Key::fromKeyCode(static_cast<int>(vk), states);
}

std::pair<std::string, std::uint32_t> readPreedit(EngineInputContext& context) {
    const auto& client = context.inputPanel().clientPreedit();
    const auto& server = context.inputPanel().preedit();
    const Text& selected = client.empty() ? server : client;
    std::string text = selected.toString();
    int cursor = selected.cursor();
    if (cursor < 0)
        cursor = static_cast<int>(text.size());
    cursor = std::clamp(cursor, 0, static_cast<int>(text.size()));
    return {std::move(text), static_cast<std::uint32_t>(cursor)};
}

} // namespace

class FcitxRuntime::Impl final {
  public:
    std::unique_ptr<Instance> instance;
    std::unordered_map<ClientContextKey, std::unique_ptr<EngineInputContext>, KeyHash> contexts;
    std::unique_ptr<EngineInputContext> warmupContext;
    EngineInputContext* focused{};
    std::uint64_t nextCompositionId{1};
    std::unordered_map<ClientContextKey, std::uint64_t, KeyHash> revisions;
    std::unordered_map<ClientContextKey, std::uint64_t, KeyHash> compositions;
    std::unordered_map<ClientContextKey, protocol::CaretRect, KeyHash> carets;
    std::unordered_map<ClientContextKey, RuntimeResult, KeyHash> pendingStates;
    // Focus override set by the Left/Right arrow keys: moves the candidate
    // highlight without committing. Cleared whenever the candidate list
    // changes (page turn, selection, new snapshot).
    std::unordered_map<ClientContextKey, std::optional<std::uint32_t>, KeyHash>
        selectedOverride;
    // Immutable snapshot of config.toml, loaded once at startup. The input
    // hot path (processKey) reads this in memory instead of reopening and
    // re-parsing the file on every keystroke.
    EngineConfig config;
    // Set when the user switched the input method on this context via the
    // Ctrl+Space / Ctrl+Shift hotkeys; the per-key reset to the group default
    // respects it so a switch survives the next keystroke.
    std::unordered_map<ClientContextKey, bool, KeyHash> inputMethodOverridden;

    void ensureInputMethods() {
        auto& manager = instance->inputMethodManager();
        manager.refresh();
        InputMethodGroup group = manager.currentGroup();
        auto& items = group.inputMethodList();
        auto contains = [&items](const std::string& name) {
            return std::any_of(items.begin(), items.end(),
                               [&](const auto& item) { return item.name() == name; });
        };
        const std::string previousDefault = group.defaultInputMethod();
        items.erase(std::remove_if(items.begin(), items.end(), [&](const auto& item) {
                        return manager.entry(item.name()) == nullptr;
                    }),
                    items.end());
        config = readEngineConfig();
        for (const std::string& id : config.enabled) {
            if (manager.entry(id) && !contains(id)) {
                items.emplace_back(id);
            }
        }
        // Preload every enabled input method addon at startup so the
        // Ctrl+Space / Ctrl+Shift switch hotkeys never pay a first-activation
        // cost inside the 100 ms TSF input deadline. Without this, switching
        // to an input method that was never focused before (e.g. Rime from a
        // pinyin default, or vice versa) can time out the key request.
        for (const std::string& id : config.enabled) {
            if (manager.entry(id)) {
                (void)instance->inputMethodEngine(id);
            }
        }
        std::string selected = previousDefault;
        if (!contains(selected)) {
            selected.clear();
            for (const std::string& fallback : config.enabled) {
                if (contains(fallback)) {
                    selected = fallback;
                    break;
                }
            }
        }
        if (selected.empty())
            return;
        const auto keyboard = std::find_if(items.begin(), items.end(), [](const auto& item) {
            return item.name() == "keyboard-us";
        });
        if (!manager.entry("keyboard-us"))
            throw std::runtime_error("Windows keyboard passthrough addon is unavailable");
        if (keyboard == items.end())
            items.insert(items.begin(), InputMethodGroupItem("keyboard-us"));
        else if (keyboard != items.begin())
            std::rotate(items.begin(), keyboard, std::next(keyboard));
        group.setDefaultInputMethod(selected);
        manager.setGroup(std::move(group));
        manager.save();
    }

    void toggleInputMethod(const EngineConfig& config, const ClientContextKey& key,
                           EngineInputContext& context) {
        auto& manager = instance->inputMethodManager();
        const std::string current = instance->inputMethod(&context);
        if (current == "keyboard-us") {
            const auto defaultName = config.defaultInputMethod.value_or("pinyin");
            if (manager.entry(defaultName))
                instance->setCurrentInputMethod(&context, defaultName, true);
        } else if (manager.entry("keyboard-us")) {
            instance->setCurrentInputMethod(&context, "keyboard-us", true);
        }
        // The per-key reset to the group default below must not undo a user
        // switch on the following keystroke.
        inputMethodOverridden[key] = true;
    }

    void nextInputMethod(const EngineConfig& config, const ClientContextKey& key,
                         EngineInputContext& context) {
        auto& manager = instance->inputMethodManager();
        const std::string current = instance->inputMethod(&context);
        if (config.enabled.empty())
            return;
        auto iter = std::find(config.enabled.begin(), config.enabled.end(), current);
        const std::string next =
            iter == config.enabled.end() ? config.enabled.front()
                                         : config.enabled[(iter - config.enabled.begin() + 1) %
                                                          config.enabled.size()];
        if (manager.entry(next)) {
            instance->setCurrentInputMethod(&context, next, true);
            inputMethodOverridden[key] = true;
        }
    }

    EngineInputContext& contextFor(const ClientContextKey& key) {
        auto found = contexts.find(key);
        if (found != contexts.end())
            return *found->second;
        auto context = std::make_unique<EngineInputContext>(instance->inputContextManager());
        auto* result = context.get();
        contexts.emplace(key, std::move(context));
        return *result;
    }

    RuntimeResult collectResult(const ClientContextKey& key,
                                EngineInputContext& context, bool handled) {
        RuntimeResult output;
        output.handled = handled;
        output.commitUtf8 = context.takeCommit();
        auto [preedit, caretOffset] = readPreedit(context);
        output.preeditUtf8 = std::move(preedit);
        output.preeditCaretUtf8 = caretOffset;
        const auto candidateList = context.inputPanel().candidateList();
        const bool hasCandidates = candidateList && !candidateList->empty();
        auto& composition = compositions[key];
        if ((!output.preeditUtf8.empty() || hasCandidates) && composition == 0) {
            composition = nextCompositionId++;
            if (composition == 0)
                composition = nextCompositionId++;
        }
        if (output.preeditUtf8.empty() && !hasCandidates)
            composition = 0;
        if (hasCandidates) {
            const int pageSize =
                std::clamp(candidateList->size(), 0,
                           static_cast<int>(protocol::kMaxCandidates));
            output.candidatePageSize = static_cast<std::uint32_t>(pageSize);
            const auto* bulk = candidateList->toBulk();
            const int reportedTotal = bulk ? bulk->totalSize() : pageSize;
            // Some addons (e.g. Rime) expose a BulkCandidateList without a
            // real bulk API: totalSize() returns -1 and globalCursorIndex()
            // returns -1. Treat them as an ordinary pageable list so the UI
            // gets page-local candidates plus a valid selection; otherwise the
            // selection would be missing and the candidate window renders
            // without any highlight.
            const bool realBulk = bulk != nullptr && reportedTotal >= 0;
            const int size =
                realBulk ? std::clamp(reportedTotal, 0,
                                      static_cast<int>(protocol::kMaxCandidates))
                         : pageSize;
            output.candidateBulk = realBulk;
            output.candidateEnd = !realBulk || (reportedTotal >= 0 && size >= reportedTotal);
            output.candidates.reserve(static_cast<std::size_t>(size));
            for (int index = 0; index < size; ++index) {
                const auto& word =
                    realBulk ? bulk->candidateFromAll(index) : candidateList->candidate(index);
                output.candidates.push_back(protocol::CandidateRecord{
                    (composition << 8U) | static_cast<std::uint64_t>(index + 1),
                    realBulk ? std::string{} : candidateList->label(index).toString(),
                    word.text().toString(), word.comment().toString()});
            }
            int cursor = candidateList->cursorIndex();
            if (const auto* bulkCursor = candidateList->toBulkCursor()) {
                const int global = bulkCursor->globalCursorIndex();
                if (global >= 0)
                    cursor = global;
            }
            if (const auto found = selectedOverride.find(key);
                found != selectedOverride.end() && found->second) {
                cursor = static_cast<int>(*found->second);
            }
            if (cursor >= 0 && cursor < size)
                output.selectedCandidate = static_cast<std::uint32_t>(cursor);
            output.candidateTotal = static_cast<std::uint32_t>(size);
            if (realBulk)
                output.candidateTotal = static_cast<std::uint32_t>(reportedTotal);
            if (const auto* pageable = candidateList->toPageable(); pageable) {
                const int page = pageable->currentPage();
                if (page >= 0)
                    output.candidatePage = static_cast<std::uint32_t>(page);
            }
            output.candidateVisibility = output.preeditUtf8.empty() ? 2U : 1U;
        }
        output.compositionId = composition;
        output.revision = ++revisions[key];
        if (const auto found = carets.find(key); found != carets.end())
            output.caret = found->second;
        return output;
    }
};

FcitxRuntime::FcitxRuntime() : impl_(std::make_unique<Impl>()) {}
FcitxRuntime::~FcitxRuntime() = default;

bool FcitxRuntime::initialize(bool safeMode) {
    try {
        if (!setupEnvironment())
            return false;
        if (safeMode) {
            char executable[] = "fcitx5-engine";
            char disable[] = "--disable=all";
            char enable[] = "--enable=windowskeyboard,pinyin,punctuation";
            char* arguments[]{executable, disable, enable};
            impl_->instance = std::make_unique<Instance>(3, arguments);
        } else {
            impl_->instance = std::make_unique<Instance>(0, nullptr);
        }
        impl_->instance->addonManager().registerDefaultLoader(nullptr);
        impl_->instance->initialize();
        impl_->ensureInputMethods();
        const auto& group = impl_->instance->inputMethodManager().currentGroup();
        const std::string selected = group.defaultInputMethod();
        if (selected.empty() || !impl_->instance->inputMethodManager().entry(selected))
            return false;
        if (!impl_->instance->inputMethodEngine(selected))
            return false;
        // Load and activate the selected engine before accepting TSF traffic.
        // This is an internal context initialization only: no synthetic user key
        // is generated, so a cold Rime session cannot consume or duplicate input.
        impl_->warmupContext =
            std::make_unique<EngineInputContext>(impl_->instance->inputContextManager());
        impl_->warmupContext->focusIn();
        impl_->instance->setCurrentInputMethod(impl_->warmupContext.get(), selected, true);
        // Fcitx engines build process-wide decoder caches on their first text
        // event. Prime those caches in an isolated context before the ready
        // signal, then reset without committing or learning any text.
        KeyEvent warmupEvent(impl_->warmupContext.get(), Key(FcitxKey_n), false);
        impl_->warmupContext->keyEvent(warmupEvent);
        impl_->warmupContext->reset();
        (void)impl_->warmupContext->takeCommit();
        impl_->focused = impl_->warmupContext.get();
        return true;
    } catch (...) {
        impl_->instance.reset();
        return false;
    }
}

::fcitx::EventLoop& FcitxRuntime::eventLoop() { return impl_->instance->eventLoop(); }

RuntimeResult FcitxRuntime::processKey(const ClientContextKey& key,
                                       const protocol::KeyRequest& request) {
    const auto currentRevision = impl_->revisions[key];
    const auto currentComposition = impl_->compositions[key];
    if (request.metadata.revision != currentRevision ||
        request.metadata.compositionId != currentComposition) {
        throw std::invalid_argument("stale input context state");
    }
    auto& context = impl_->contextFor(key);
    if (impl_->focused != &context) {
        if (impl_->focused && impl_->focused->hasFocus())
            impl_->focused->focusOut();
        context.focusIn();
        impl_->focused = &context;
    }
    const std::string selected =
        impl_->instance->inputMethodManager().currentGroup().defaultInputMethod();
    if (!impl_->inputMethodOverridden[key] && !selected.empty() &&
        impl_->instance->inputMethodManager().entry(selected) &&
        impl_->instance->inputMethod(&context) != selected) {
        (void)impl_->instance->inputMethodEngine(selected);
        impl_->instance->setCurrentInputMethod(&context, selected, true);
    }
    KeyEvent event(&context, keyFromRequest(request),
                   (request.keyFlags & protocol::kKeyFlagRelease) != 0);
    // Input-method switch hotkeys from [hotkeys] in config.toml. The toggle
    // flips between the active input method and keyboard passthrough; next
    // cycles through [input_methods].enabled.
    if (!event.isRelease()) {
        const auto& keySym = event.key().sym();
        const auto states = event.key().states();
        const bool ctrl = states.test(KeyState::Ctrl);
        const bool shift = states.test(KeyState::Shift);
        const bool alt = states.test(KeyState::Alt);
        // Read the immutable config snapshot loaded at startup - never reopen
        // config.toml on the input hot path.
        const auto& engineConfig = impl_->config;
        const auto matches = [&](const std::optional<std::string>& hotkey) {
            if (!hotkey || keySym == FcitxKey_None)
                return false;
            if (*hotkey == "Ctrl+Space")
                return ctrl && !shift && !alt && keySym == FcitxKey_space;
            if (*hotkey == "Ctrl+Shift")
                return ctrl && shift && !alt && keySym == FcitxKey_Shift_L;
            if (*hotkey == "Ctrl+Shift+Space")
                return ctrl && shift && !alt && keySym == FcitxKey_space;
            if (*hotkey == "Alt+Shift")
                return alt && shift && !ctrl && keySym == FcitxKey_Shift_L;
            return false;
        };
        if (matches(engineConfig.hotkeyToggle)) {
            impl_->toggleInputMethod(engineConfig, key, context);
            event.filterAndAccept();
            return impl_->collectResult(key, context, true);
        }
        if (matches(engineConfig.hotkeyNext)) {
            impl_->nextInputMethod(engineConfig, key, context);
            event.filterAndAccept();
            return impl_->collectResult(key, context, true);
        }
    }
    // Candidate navigation keys while candidates are visible. Fcitx's default
    // PrevPage/NextPage are Up/Down, which the scroll viewport uses for
    // continuous cursor movement, so route the number-row page keys and the
    // comma/period and bracket pairs to the pageable list explicitly, compare
    // key symbols directly ('+' and '_' carry a Shift state), select the
    // second/third candidate with ';'/''', and move the highlight with the
    // Left/Right arrow keys without committing.
    {
        const KeySym sym = event.key().sym();
        if (!event.isRelease()) {
            if (const auto list = context.inputPanel().candidateList();
                list && !list->empty()) {
                const auto* bulk = list->toBulk();
                const int count =
                    bulk && bulk->totalSize() >= 0 ? bulk->totalSize() : list->size();
                const int bounded = std::clamp(
                    count, 0, static_cast<int>(protocol::kMaxCandidates));
                if (auto* pageable = list->toPageable(); pageable) {
                    const bool nextPage =
                        sym == FcitxKey_equal || sym == FcitxKey_plus ||
                        sym == FcitxKey_period || sym == FcitxKey_bracketright;
                    const bool prevPage =
                        sym == FcitxKey_minus || sym == FcitxKey_underscore ||
                        sym == FcitxKey_comma || sym == FcitxKey_bracketleft;
                    if (!nextPage && !prevPage)
                        goto page_keys_done;
                    const bool scroll = bulk && bulk->totalSize() >= 0;
                    if (scroll) {
                        // Scroll viewport: '+'/'-' move one row (the fixed
                        // 6-column grid) and land the highlight on the FIRST
                        // candidate of the new row. While the scroll panel is
                        // not yet open (still on page 0, which is how the UI
                        // decides to expand it), first turn the page so the
                        // candidatePage advances and the viewport opens; once
                        // paged, the row moves inside the fetched candidates
                        // without paging again.
                        constexpr int columns = 6;
                        const int page = pageable->currentPage();
                        const int available = bounded;
                        if (page <= 0 && nextPage && pageable->hasNext()) {
                            pageable->next();
                            event.filter();
                            impl_->selectedOverride[key] = 0;
                        } else if (page <= 0 && prevPage && pageable->hasPrev()) {
                            pageable->prev();
                            event.filter();
                            impl_->selectedOverride[key] = 0;
                        } else {
                            int cursor = 0;
                            if (const auto found = impl_->selectedOverride.find(key);
                                found != impl_->selectedOverride.end() && found->second) {
                                cursor = static_cast<int>(*found->second);
                            } else if (const auto* bulkCursor = list->toBulkCursor()) {
                                const int global = bulkCursor->globalCursorIndex();
                                if (global >= 0)
                                    cursor = global;
                            } else {
                                cursor = list->cursorIndex();
                            }
                            cursor = std::clamp(cursor, 0, (std::max)(0, available - 1));
                            const int rowStart = (cursor / columns) * columns;
                            int target = rowStart + (nextPage ? columns : -columns);
                            if (target < 0 || target >= available) {
                                if (target < 0 && pageable->hasPrev()) {
                                    pageable->prev();
                                    event.filter();
                                } else if (target >= available && pageable->hasNext()) {
                                    pageable->next();
                                    event.filter();
                                }
                                impl_->selectedOverride[key] = 0;
                            } else {
                                impl_->selectedOverride[key] =
                                    static_cast<std::uint32_t>(target);
                                event.filter();
                            }
                        }
                    } else if (nextPage && pageable->hasNext()) {
                        pageable->next();
                        event.filter();
                        // The highlight jumps to the first candidate of the
                        // new page instead of keeping the previous position
                        // (Fcitx's cursor can stay within the page column).
                        impl_->selectedOverride[key] = 0;
                    } else if (prevPage && pageable->hasPrev()) {
                        pageable->prev();
                        event.filter();
                        impl_->selectedOverride[key] = 0;
                    }
                }
            page_keys_done:;
                // ';' selects the second candidate, '\'' the third, on top of
                // the digit keys Fcitx already handles.
                if (sym == FcitxKey_semicolon || sym == FcitxKey_apostrophe) {
                    const std::size_t target = sym == FcitxKey_semicolon ? 1U : 2U;
                    if (target < static_cast<std::size_t>(bounded)) {
                        const auto& candidate =
                            bulk && bulk->totalSize() >= 0
                                ? bulk->candidateFromAll(static_cast<int>(target))
                                : list->candidate(static_cast<int>(target));
                        candidate.select(&context);
                        event.filterAndAccept();
                        impl_->selectedOverride.erase(key);
                        return impl_->collectResult(key, context, true);
                    }
                }
                // Left/Right move the highlight without committing.
                if (sym == FcitxKey_Left || sym == FcitxKey_Right) {
                    if (bounded > 0) {
                        int focus = 0;
                        if (const auto found = impl_->selectedOverride.find(key);
                            found != impl_->selectedOverride.end() && found->second) {
                            focus = static_cast<int>(*found->second);
                        } else {
                            focus = list->cursorIndex();
                        }
                        focus += sym == FcitxKey_Right ? 1 : -1;
                        focus = std::clamp(focus, 0, bounded - 1);
                        impl_->selectedOverride[key] =
                            static_cast<std::uint32_t>(focus);
                        event.filterAndAccept();
                        return impl_->collectResult(key, context, true);
                    }
                }
            }
        }
    }
    context.keyEvent(event);
    impl_->carets[key] = request.caret;
    return impl_->collectResult(key, context, event.accepted());
}

RuntimeResult FcitxRuntime::selectCandidate(
    std::uint32_t targetProcessId,
    const protocol::CandidateSelectRequest& request) {
    const auto found = std::find_if(
        impl_->contexts.begin(), impl_->contexts.end(), [&](const auto& item) {
            return item.first.processId == targetProcessId &&
                   item.first.contextId == request.metadata.contextId;
        });
    if (found == impl_->contexts.end() ||
        impl_->revisions[found->first] != request.metadata.revision ||
        impl_->compositions[found->first] != request.metadata.compositionId ||
        request.candidateId == 0 || (request.candidateId >> 8U) != request.metadata.compositionId) {
        throw std::invalid_argument("stale candidate selection state");
    }
    auto& context = *found->second;
    const auto candidateList = context.inputPanel().candidateList();
    const std::uint64_t encodedIndex = request.candidateId & 0xffU;
    if (!candidateList || encodedIndex == 0)
        throw std::invalid_argument("candidate selection is unavailable");
    const std::size_t index = static_cast<std::size_t>(encodedIndex - 1U);
    const auto* bulk = candidateList->toBulk();
    const int count = bulk && bulk->totalSize() >= 0 ? bulk->totalSize() : candidateList->size();
    if (index >= static_cast<std::size_t>(std::clamp(
                     count, 0, static_cast<int>(protocol::kMaxCandidates)))) {
        throw std::invalid_argument("candidate selection index is invalid");
    }
    const auto& candidate = bulk ? bulk->candidateFromAll(static_cast<int>(index))
                                 : candidateList->candidate(static_cast<int>(index));
    candidate.select(&context);
    RuntimeResult output = impl_->collectResult(found->first, context, true);
    impl_->pendingStates[found->first] = output;
    return output;
}

RuntimeResult FcitxRuntime::takePendingState(
    const ClientContextKey& key, const protocol::StateRequest& request) {
    const auto found = impl_->pendingStates.find(key);
    if (found == impl_->pendingStates.end() ||
        request.metadata.revision >= found->second.revision) {
        throw std::invalid_argument("pending state is unavailable");
    }
    RuntimeResult output = std::move(found->second);
    impl_->pendingStates.erase(found);
    return output;
}

std::vector<InputMethodInfo> FcitxRuntime::inputMethods() const {
    std::vector<InputMethodInfo> output;
    if (!impl_->instance)
        return output;
    const auto& manager = impl_->instance->inputMethodManager();
    const auto& group = manager.currentGroup();
    output.reserve(group.inputMethodList().size());
    for (const auto& item : group.inputMethodList()) {
        const auto* entry = manager.entry(item.name());
        if (!entry || entry->uniqueName() == "keyboard-us")
            continue;
        output.push_back(InputMethodInfo{entry->uniqueName(), entry->name(), entry->nativeName(),
                                         entry->uniqueName() == group.defaultInputMethod()});
    }
    return output;
}

bool FcitxRuntime::setDefaultInputMethod(std::string_view id) {
    if (!impl_->instance || id.empty() || id == "keyboard-us")
        return false;
    auto& manager = impl_->instance->inputMethodManager();
    InputMethodGroup group = manager.currentGroup();
    const bool present = std::any_of(group.inputMethodList().begin(), group.inputMethodList().end(),
                                     [&](const auto& item) { return item.name() == id; });
    if (!present || !manager.entry(std::string(id)))
        return false;
    auto& items = group.inputMethodList();
    const auto keyboard = std::find_if(items.begin(), items.end(), [](const auto& item) {
        return item.name() == "keyboard-us";
    });
    if (!manager.entry("keyboard-us"))
        return false;
    if (keyboard == items.end())
        items.insert(items.begin(), InputMethodGroupItem("keyboard-us"));
    else if (keyboard != items.begin())
        std::rotate(items.begin(), keyboard, std::next(keyboard));
    group.setDefaultInputMethod(std::string(id));
    manager.setGroup(std::move(group));
    manager.save();
    return manager.currentGroup().defaultInputMethod() == id;
}

void FcitxRuntime::forgetConnection(std::uint64_t connectionId) {
    for (auto iterator = impl_->contexts.begin(); iterator != impl_->contexts.end();) {
        if (iterator->first.connectionId == connectionId) {
            if (impl_->focused == iterator->second.get())
                impl_->focused = nullptr;
            impl_->revisions.erase(iterator->first);
            impl_->compositions.erase(iterator->first);
            impl_->carets.erase(iterator->first);
            impl_->pendingStates.erase(iterator->first);
            impl_->inputMethodOverridden.erase(iterator->first);
            impl_->selectedOverride.erase(iterator->first);
            iterator = impl_->contexts.erase(iterator);
        } else {
            ++iterator;
        }
    }
}

} // namespace fcitx::windows::engine
