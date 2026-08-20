#include "fcitx_runtime.h"
#include "candidate_navigation.h"
#include "config_model.h"
#include "runtime_identity.h"
#include <fcitx5_windows/release_identity.h>

#include <fcitx-utils/capabilityflags.h>
#include <fcitx-utils/environ.h>
#include <fcitx-utils/key.h>
#include <fcitx-utils/keysym.h>
#include <fcitx/addonmanager.h>
#include <fcitx/candidatelist.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputmethodengine.h>
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
    config::Orientation orientation{config::Orientation::vertical};
    std::optional<int> candidatePageSize;
    bool scrollMode{};
};

std::filesystem::path localDataDirectory() {
    std::wstring modulePath(32'768, L'\0');
    const DWORD size =
        GetModuleFileNameW(nullptr, modulePath.data(), static_cast<DWORD>(modulePath.size()));
    if (size > 0 && size < modulePath.size()) {
        modulePath.resize(size);
        if (const auto portableData =
                fcitx::windows::platform::portableDataRootForModule(modulePath);
            !portableData.empty()) {
            return portableData;
        }
    }
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
        // Missing or broken user config must not implicitly enable every
        // bundled/input-method profile. Keep the engine's default activation
        // policy aligned with TSF registration: pinyin only until the user or
        // package config explicitly enables additional engines such as Rime.
        parsed.enabledInputMethods = {"pinyin"};
        parsed.defaultInputMethod = "pinyin";
    }
    config.enabled = parsed.enabledInputMethods;
    config.defaultInputMethod = parsed.defaultInputMethod;
    config.hotkeyToggle = parsed.hotkeyToggleInputMethod;
    config.hotkeyNext = parsed.hotkeyNextInputMethod;
    config.orientation =
        parsed.orientation.value_or(config::Orientation::vertical);
    config.candidatePageSize = parsed.candidatePageSize;
    config.scrollMode = parsed.scrollMode.value_or(false);
    if (config.enabled.empty())
        config.enabled = {"pinyin"};
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
    if (!getEnvironment("FCITX_USER_DATA_ROOT")) {
        const auto portableRoot =
            fcitx::windows::platform::portableDataRootForModule(modulePath);
        if (!portableRoot.empty()) {
            const auto portableData = utf8Path(portableRoot);
            if (portableData.empty())
                return false;
            setEnvironment("FCITX_USER_DATA_ROOT", portableData.c_str());
        }
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
    struct DeleteSurroundingOperation {
        std::int32_t offset{};
        std::uint32_t size{};
    };

    struct ForwardKeyOperation {
        std::uint32_t sym{};
        std::uint32_t states{};
        std::int32_t code{};
        bool release{};
    };

    explicit EngineInputContext(InputContextManager& manager) : InputContext(manager, "tsf") {
        setEnablePreedit(true);
        setCapabilityFlags(CapabilityFlags{CapabilityFlag::Preedit,
                                           CapabilityFlag::SurroundingText,
                                           CapabilityFlag::ClientSideInputPanel});
        created();
    }

    ~EngineInputContext() override { destroy(); }

    const char* frontend() const override { return "tsf"; }

    std::string takeCommit() { return std::exchange(commit_, {}); }
    std::optional<DeleteSurroundingOperation> takeDeleteSurroundingText() {
        return std::exchange(deleteSurroundingText_, std::nullopt);
    }
    std::optional<ForwardKeyOperation> takeForwardKey() {
        return std::exchange(forwardKey_, std::nullopt);
    }
    bool applySurroundingText(const protocol::KeyRequest& request) {
        if (request.surroundingTextValid) {
            surroundingText().setText(request.surroundingTextUtf8,
                                      request.surroundingCursor,
                                      request.surroundingAnchor);
            surroundingTextValid_ = true;
            return true;
        }
        if (surroundingTextValid_) {
            surroundingText().invalidate();
            surroundingTextValid_ = false;
            return true;
        }
        surroundingText().invalidate();
        return false;
    }

  private:
    void commitStringImpl(const std::string& text) override { commit_ += text; }
    void forwardKeyImpl(const ForwardKeyEvent& key) override {
        const auto rawKey = key.rawKey();
        forwardKey_ = ForwardKeyOperation{
            static_cast<std::uint32_t>(rawKey.sym()),
            static_cast<std::uint32_t>(rawKey.states().toInteger()),
            rawKey.code(),
            key.isRelease()};
    }
    void deleteSurroundingTextImpl(int offset, unsigned int size) override {
        deleteSurroundingText_ = DeleteSurroundingOperation{
            static_cast<std::int32_t>(offset), static_cast<std::uint32_t>(size)};
    }
    void updatePreeditImpl() override {}

    std::string commit_;
    std::optional<DeleteSurroundingOperation> deleteSurroundingText_;
    std::optional<ForwardKeyOperation> forwardKey_;
    bool surroundingTextValid_{};
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

std::size_t utf8CharacterEnd(std::string_view text, std::size_t offset) noexcept {
    if (offset >= text.size())
        return text.size();
    const auto byte = static_cast<unsigned char>(text[offset]);
    std::size_t length = 1;
    if ((byte & 0xe0U) == 0xc0U)
        length = 2;
    else if ((byte & 0xf0U) == 0xe0U)
        length = 3;
    else if ((byte & 0xf8U) == 0xf0U)
        length = 4;
    return (std::min)(text.size(), offset + length);
}

std::string statusShortLabel(std::string_view text) {
    if (text.empty())
        return {};
    if (text.size() >= 2 &&
        static_cast<unsigned char>(text[0]) < 0x80U &&
        static_cast<unsigned char>(text[1]) < 0x80U) {
        return std::string(text.substr(0, 2));
    }
    return std::string(text.substr(0, utf8CharacterEnd(text, 0)));
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
    // Focus override set by row/page navigation: moves the candidate highlight
    // without committing. Cleared whenever ordinary input or selection changes
    // the candidate state.
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
        for (auto iterator = contexts.begin(); iterator != contexts.end();) {
            if (iterator->first.processId == key.processId &&
                iterator->first.contextId == key.contextId &&
                iterator->first.connectionId != key.connectionId) {
                iterator = eraseContext(iterator);
            } else {
                ++iterator;
            }
        }
        auto context = std::make_unique<EngineInputContext>(instance->inputContextManager());
        auto* result = context.get();
        contexts.emplace(key, std::move(context));
        return *result;
    }

    decltype(contexts)::iterator eraseContext(decltype(contexts)::iterator iterator) {
        auto* context = iterator->second.get();
        try {
            context->reset();
            if (context->hasFocus())
                context->focusOut();
        } catch (...) {
        }
        if (focused == context)
            focused = nullptr;
        revisions.erase(iterator->first);
        compositions.erase(iterator->first);
        carets.erase(iterator->first);
        pendingStates.erase(iterator->first);
        inputMethodOverridden.erase(iterator->first);
        selectedOverride.erase(iterator->first);
        return contexts.erase(iterator);
    }

    void applyFcitxPageSize() {
        if (!config.candidatePageSize)
            return;
        RawConfig raw;
        instance->globalConfig().save(raw);
        raw.setValueByPath("Behavior/DefaultPageSize", std::to_string(*config.candidatePageSize));
        instance->globalConfig().load(raw, true);
    }

    void dispatchPendingEvents() {
        if (instance)
            instance->eventDispatcher().dispatchPending();
    }

    RuntimeResult collectResult(const ClientContextKey& key,
                                EngineInputContext& context, bool handled) {
        dispatchPendingEvents();
        RuntimeResult output;
        output.handled = handled;
        output.commitUtf8 = context.takeCommit();
        auto [preedit, caretOffset] = readPreedit(context);
        output.preeditUtf8 = std::move(preedit);
        output.preeditCaretUtf8 = caretOffset;
        if (const auto operation = context.takeDeleteSurroundingText()) {
            output.deleteSurroundingText = true;
            output.deleteSurroundingOffset = operation->offset;
            output.deleteSurroundingSize = operation->size;
        }
        if (const auto operation = context.takeForwardKey()) {
            output.forwardKey = true;
            output.forwardKeySym = operation->sym;
            output.forwardKeyStates = operation->states;
            output.forwardKeyCode = operation->code;
            output.forwardKeyRelease = operation->release;
        }
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
            const int fcitxPageSize =
                std::clamp(candidateList->size(), 0,
                           static_cast<int>(protocol::kMaxCandidates));
            const auto* bulk = candidateList->toBulk();
            const int reportedTotal = bulk ? bulk->totalSize() : fcitxPageSize;
            // Some addons (e.g. Rime) expose a BulkCandidateList without a
            // real bulk API: totalSize() returns -1 and globalCursorIndex()
            // returns -1. Treat them as an ordinary pageable list so the UI
            // gets page-local candidates plus a valid selection; otherwise the
            // selection would be missing and the candidate window renders
            // without any highlight.
            const bool realBulk = bulk != nullptr && reportedTotal >= 0;
            const int pageSize =
                realBulk
                    ? std::clamp(config.candidatePageSize.value_or(fcitxPageSize), 1,
                                 static_cast<int>(protocol::kMaxCandidates))
                    : fcitxPageSize;
            output.candidatePageSize = static_cast<std::uint32_t>(pageSize);
            const int size =
                realBulk ? std::clamp(reportedTotal, 0,
                                      static_cast<int>(protocol::kMaxCandidates))
                         : fcitxPageSize;
            output.candidateBulk = realBulk;
            output.candidateEnd = !realBulk || (reportedTotal >= 0 && size >= reportedTotal);
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
            int page = 0;
            if (const auto* pageable = candidateList->toPageable(); pageable) {
                page = (std::max)(0, pageable->currentPage());
                output.candidatePage = static_cast<std::uint32_t>(page);
            }
            const int dimension = (std::max)(1, pageSize);
            output.candidates.reserve(static_cast<std::size_t>(size));
            for (int index = 0; index < size; ++index) {
                const auto& word =
                    realBulk ? bulk->candidateFromAll(index) : candidateList->candidate(index);
                std::string label;
                if (!realBulk) {
                    label = candidateList->label(index).toString();
                } else if (config.scrollMode) {
                    const auto offset =
                        config.orientation == config::Orientation::vertical
                            ? columnSelectionRow(static_cast<std::size_t>((std::max)(0, cursor)),
                                                 static_cast<std::size_t>(index),
                                                 static_cast<std::size_t>(dimension),
                                                 static_cast<std::size_t>(size))
                            : rowSelectionColumn(static_cast<std::size_t>((std::max)(0, cursor)),
                                                 static_cast<std::size_t>(index),
                                                 static_cast<std::size_t>(dimension),
                                                 static_cast<std::size_t>(size));
                    if (offset) {
                        label = std::to_string(*offset + 1U);
                    }
                } else if (index >= page * dimension && index < (page + 1) * dimension) {
                    label = std::to_string(index - page * dimension + 1);
                }
                output.candidates.push_back(protocol::CandidateRecord{
                    (composition << 8U) | static_cast<std::uint64_t>(index + 1),
                    std::move(label), word.text().toString(), word.comment().toString()});
            }
            output.candidateTotal = static_cast<std::uint32_t>(size);
            if (realBulk)
                output.candidateTotal = static_cast<std::uint32_t>(reportedTotal);
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
        impl_->applyFcitxPageSize();
        const auto& group = impl_->instance->inputMethodManager().currentGroup();
        const std::string selected = group.defaultInputMethod();
        if (selected.empty() || !impl_->instance->inputMethodManager().entry(selected))
            return false;
        if (!impl_->instance->inputMethodEngine(selected))
            return false;
        // Load and activate the enabled engines before accepting TSF traffic.
        // This is an internal context initialization only: no synthetic text key
        // is sent. Addons that need deeper preload must expose an explicit,
        // side-effect-free preload hook; generic warmup must not learn,
        // commit, mutate history, or consume what looks like a real user key.
        impl_->warmupContext =
            std::make_unique<EngineInputContext>(impl_->instance->inputContextManager());
        impl_->warmupContext->focusIn();
        std::vector<std::string> warmupIds = impl_->config.enabled;
        if (std::find(warmupIds.begin(), warmupIds.end(), selected) == warmupIds.end())
            warmupIds.push_back(selected);
        for (const auto& id : warmupIds) {
            if (!impl_->instance->inputMethodManager().entry(id))
                continue;
            (void)impl_->instance->inputMethodEngine(id);
            impl_->instance->setCurrentInputMethod(impl_->warmupContext.get(), id, true);
            impl_->dispatchPendingEvents();
            impl_->warmupContext->reset();
            (void)impl_->warmupContext->takeCommit();
        }
        impl_->instance->setCurrentInputMethod(impl_->warmupContext.get(), selected, true);
        impl_->dispatchPendingEvents();
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
        impl_->dispatchPendingEvents();
    }
    if (context.applySurroundingText(request)) {
        context.updateSurroundingText();
    }
    const auto& group = impl_->instance->inputMethodManager().currentGroup();
    const std::string selected =
        !request.inputMethodUtf8.empty() &&
                impl_->instance->inputMethodManager().entry(request.inputMethodUtf8)
            ? request.inputMethodUtf8
            : group.defaultInputMethod();
    if (!impl_->inputMethodOverridden[key] && !selected.empty() &&
        impl_->instance->inputMethodManager().entry(selected) &&
        impl_->instance->inputMethod(&context) != selected) {
        (void)impl_->instance->inputMethodEngine(selected);
        impl_->instance->setCurrentInputMethod(&context, selected, true);
        impl_->dispatchPendingEvents();
    }
    KeyEvent event(&context, keyFromRequest(request),
                   (request.keyFlags & protocol::kKeyFlagRelease) != 0);
    // Input-method switch hotkeys from [hotkeys] in config.toml. The toggle
    // flips between the active input method and keyboard passthrough; next
    // cycles through [input_methods].enabled.
    if (!event.isRelease()) {
        const auto& keySym = event.key().sym();
        const bool ctrl = (request.keyFlags & protocol::kKeyFlagControl) != 0;
        const bool shift = (request.keyFlags & protocol::kKeyFlagShift) != 0;
        const bool alt = (request.keyFlags & protocol::kKeyFlagAlt) != 0;
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
                const bool nextPage =
                    sym == FcitxKey_equal || sym == FcitxKey_plus ||
                    sym == FcitxKey_period || sym == FcitxKey_bracketright;
                const bool prevPage =
                    sym == FcitxKey_minus || sym == FcitxKey_underscore ||
                    sym == FcitxKey_comma || sym == FcitxKey_bracketleft;
                const bool upDown = sym == FcitxKey_Up || sym == FcitxKey_Down;
                const bool scroll = impl_->config.scrollMode && bulk && bulk->totalSize() >= 0;
                const bool verticalScroll =
                    impl_->config.orientation == config::Orientation::vertical;
                const std::size_t dimension = static_cast<std::size_t>(
                    std::clamp(impl_->config.candidatePageSize.value_or(list->size()), 1,
                               static_cast<int>(protocol::kMaxCandidates)));
                // In the scroll viewport, Up/Down scroll one row instead of
                // the Fcitx default page turn; in ordinary paging they keep
                // the Fcitx default (Up=PrevPage, Down=NextPage).
                const bool scrollNext = nextPage || (scroll && sym == FcitxKey_Down);
                const bool scrollPrev = prevPage || (scroll && sym == FcitxKey_Up);
                const auto currentFocus = [&] {
                    int focus = 0;
                    if (const auto found = impl_->selectedOverride.find(key);
                        found != impl_->selectedOverride.end() && found->second) {
                        focus = static_cast<int>(*found->second);
                    } else if (const auto* bulkCursor = list->toBulkCursor()) {
                        const int global = bulkCursor->globalCursorIndex();
                        focus = global >= 0 ? global : list->cursorIndex();
                    } else {
                        focus = list->cursorIndex();
                    }
                    return std::clamp(focus, 0, (std::max)(0, bounded - 1));
                };
                const bool plainShortcut =
                    (request.keyFlags & (protocol::kKeyFlagShift |
                                         protocol::kKeyFlagControl |
                                         protocol::kKeyFlagAlt)) == 0;
                if (scroll && plainShortcut) {
                    std::optional<std::size_t> column;
                    bool consume = false;
                    if (sym == FcitxKey_0) {
                        consume = true;
                    } else if (sym >= FcitxKey_1 && sym <= FcitxKey_9) {
                        consume = true;
                        const auto digit = static_cast<std::size_t>(sym - FcitxKey_1);
                        if (digit < dimension)
                            column = digit;
                    } else if (sym == FcitxKey_semicolon) {
                        consume = true;
                        column = 1U;
                    } else if (sym == FcitxKey_apostrophe) {
                        consume = true;
                        column = 2U;
                    }
                    if (consume) {
                        const auto target =
                            column ? (verticalScroll
                                          ? columnSelectionTarget(
                                                static_cast<std::size_t>(currentFocus()), *column,
                                                dimension, static_cast<std::size_t>(bounded))
                                          : rowSelectionTarget(
                                                static_cast<std::size_t>(currentFocus()), *column,
                                                dimension, static_cast<std::size_t>(bounded)))
                                   : std::nullopt;
                        if (target) {
                            bulk->candidateFromAll(static_cast<int>(*target)).select(&context);
                            impl_->selectedOverride.erase(key);
                        }
                        event.filterAndAccept();
                        return impl_->collectResult(key, context, true);
                    }
                }
                auto* pageable = list->toPageable();
                if (scroll && (scrollNext || scrollPrev)) {
                    // Scroll viewport (bulk candidate list). Horizontal
                    // layout is a row grid: Up/Down keep the column, while
                    // page keys land on the row start. Vertical layout is a
                    // single visible column: Up/Down move within that column,
                    // while page keys switch to the previous/next column top.
                    const int rowWidth = static_cast<int>(dimension);
                    const int available = (std::max)(0, bounded);
                    const int cursor = currentFocus();
                    const auto navigation =
                        verticalScroll && upDown
                            ? sameColumnNavigationTarget(static_cast<std::size_t>(cursor),
                                                         dimension,
                                                         static_cast<std::size_t>(available),
                                                         scrollNext)
                        : verticalScroll
                            ? columnNavigationTarget(static_cast<std::size_t>(cursor),
                                                     dimension,
                                                     static_cast<std::size_t>(available),
                                                     scrollNext, false)
                            : rowNavigationTarget(static_cast<std::size_t>(cursor),
                                                  dimension,
                                                  static_cast<std::size_t>(available),
                                                  scrollNext, upDown);
                    int target = static_cast<int>(navigation.index);
                    if (navigation.beforeStart) {
                        if (!(verticalScroll && upDown) && pageable && pageable->hasPrev()) {
                            pageable->prev();
                            event.filter();
                            target = !verticalScroll && upDown ? cursor % rowWidth : 0;
                        }
                    } else if (navigation.afterEnd) {
                        if (!(verticalScroll && upDown) && pageable && pageable->hasNext()) {
                            pageable->next();
                            event.filter();
                            target = !verticalScroll && upDown ? cursor % rowWidth : 0;
                        }
                    }
                    impl_->selectedOverride[key] =
                        static_cast<std::uint32_t>(std::clamp(
                            target, 0, (std::max)(0, available - 1)));
                    event.filterAndAccept();
                    return impl_->collectResult(key, context, true);
                }
                if (pageable && (nextPage || prevPage)) {
                    if (nextPage && pageable->hasNext()) {
                        pageable->next();
                        // The highlight jumps to the first candidate of the
                        // new page instead of keeping the previous position
                        // (Fcitx's cursor can stay within the page column).
                        impl_->selectedOverride[key] = 0;
                    } else if (prevPage && pageable->hasPrev()) {
                        pageable->prev();
                        impl_->selectedOverride[key] = 0;
                    }
                    event.filterAndAccept();
                    return impl_->collectResult(key, context, true);
                }
                // In ordinary paging, ';' selects the second candidate and
                // '\'' the third. Scroll mode handled them above relative to
                // the highlighted row so their semantics match labels 2/3.
                if (!scroll &&
                    (sym == FcitxKey_semicolon || sym == FcitxKey_apostrophe)) {
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
                // Left/Right move the highlighted candidate without
                // committing, but only while a candidate list is present. TSF
                // keeps idle navigation in the host editor, so this does not
                // freeze caret movement outside real IME input.
                if (plainShortcut && (sym == FcitxKey_Left || sym == FcitxKey_Right) &&
                    bounded > 0) {
                    const int focus = currentFocus();
                    int nextFocus = focus;
                    if (scroll && verticalScroll) {
                        const auto navigation =
                            columnNavigationTarget(static_cast<std::size_t>(focus),
                                                   dimension,
                                                   static_cast<std::size_t>(bounded),
                                                   sym == FcitxKey_Right, true);
                        nextFocus = static_cast<int>(navigation.index);
                    } else {
                        nextFocus = std::clamp(
                            focus + (sym == FcitxKey_Right ? 1 : -1), 0, bounded - 1);
                    }
                    impl_->selectedOverride[key] =
                        static_cast<std::uint32_t>(nextFocus);
                    event.filterAndAccept();
                    return impl_->collectResult(key, context, true);
                }
                // Space / Enter commit the highlighted candidate (the
                // selectedOverride set by the arrow/page keys) instead of the
                // Fcitx cursor, which may still point at the first candidate
                // after the highlight moved. Without this the committed text
                // does not match the highlighted row.
                if ((sym == FcitxKey_space || sym == FcitxKey_Return) &&
                    !event.isRelease()) {
                    if (const auto found = impl_->selectedOverride.find(key);
                        found != impl_->selectedOverride.end() && found->second) {
                        const int focus = static_cast<int>(*found->second);
                        if (focus >= 0 && focus < bounded) {
                            const auto& candidate =
                                bulk && bulk->totalSize() >= 0
                                    ? bulk->candidateFromAll(focus)
                                    : list->candidate(focus);
                            candidate.select(&context);
                            event.filterAndAccept();
                            impl_->selectedOverride.erase(key);
                            return impl_->collectResult(key, context, true);
                        }
                    }
                }
            }
        }
    }
    if (!event.isRelease() && event.key().sym() != FcitxKey_Shift_L &&
        event.key().sym() != FcitxKey_Control_L &&
        event.key().sym() != FcitxKey_Alt_L) {
        impl_->selectedOverride.erase(key);
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
    impl_->selectedOverride.erase(found->first);
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

InputMethodStatus FcitxRuntime::currentInputMethod() const {
    InputMethodStatus output;
    if (!impl_->instance)
        return output;
    auto* context = impl_->focused ? impl_->focused : impl_->warmupContext.get();
    auto& manager = impl_->instance->inputMethodManager();
    const std::string id = context ? impl_->instance->inputMethod(context)
                                   : manager.currentGroup().defaultInputMethod();
    if (id.empty())
        return output;
    output.id = id;
    const auto* entry = manager.entry(id);
    if (!entry) {
        output.name = id;
        output.shortLabel = statusShortLabel(output.name);
        return output;
    }
    output.name = entry->name();
    output.nativeName = entry->nativeName();
    std::string display;
    if (context) {
        if (auto* engine = impl_->instance->inputMethodEngine(context)) {
            display = engine->subModeLabel(*entry, *context);
        }
    }
    if (display.empty())
        display = entry->label().empty() ? output.nativeName : entry->label();
    if (display.empty())
        display = output.nativeName.empty() ? output.name : output.nativeName;
    output.shortLabel = statusShortLabel(display);
    return output;
}

bool FcitxRuntime::setDefaultInputMethod(std::string_view id) {
    if (!impl_->instance || id.empty() || id == "keyboard-us")
        return false;
    auto& manager = impl_->instance->inputMethodManager();
    InputMethodGroup group = manager.currentGroup();
    if (!manager.entry(std::string(id)))
        return false;
    auto& items = group.inputMethodList();
    const bool present = std::any_of(items.begin(), items.end(),
                                     [&](const auto& item) { return item.name() == id; });
    if (!present)
        items.emplace_back(std::string(id));
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
            iterator = impl_->eraseContext(iterator);
        } else {
            ++iterator;
        }
    }
}

} // namespace fcitx::windows::engine
