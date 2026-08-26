#include "fcitx_runtime.h"
#include "config_model.h"
#include "engine_core_ffi.h"
#include "key_event.h"
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
#include <set>
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

template <typename Function>
Function resolveProcAddress(HMODULE module, const char* name) noexcept {
#if defined(__clang__)
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wcast-function-type-mismatch"
#endif
    const auto function = reinterpret_cast<Function>(GetProcAddress(module, name));
#if defined(__clang__)
#pragma clang diagnostic pop
#endif
    return function;
}

std::filesystem::path pathFromUtf8(std::string_view value) {
    if (value.empty())
        return {};
    const int size =
        MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, value.data(),
                            static_cast<int>(value.size()), nullptr, 0);
    if (size <= 0)
        return {};
    std::wstring wide(static_cast<std::size_t>(size), L'\0');
    return MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, value.data(),
                               static_cast<int>(value.size()), wide.data(), size) == size
               ? std::filesystem::path(wide)
               : std::filesystem::path{};
}

std::filesystem::path localDataDirectory() {
    if (const auto dataRoot = getEnvironment("FCITX_USER_DATA_ROOT");
        dataRoot && !dataRoot->empty()) {
        return pathFromUtf8(*dataRoot) / "Fcitx5";
    }
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
    const auto parsedOrientation = parsed.orientation.value_or(config::Orientation::vertical);
    config.orientation = parsedOrientation == config::Orientation::automatic
                             ? config::Orientation::vertical
                             : parsedOrientation;
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

std::string startupAddonList(const EngineConfig& config) {
    std::set<std::string> addons{
        "windowskeyboard",
        "pinyin",
        "punctuation",
        "pinyinhelper",
        "chttrans",
        "luaaddonloader",
        "imeapi",
    };
    for (const auto& id : config.enabled) {
        if (id == "pinyin") {
            addons.insert("pinyin");
            addons.insert("punctuation");
            addons.insert("pinyinhelper");
        } else if (id == "rime" || id.starts_with("rime-")) {
            addons.insert("rime");
        } else if (id == "wbx" || id == "table" || id.starts_with("table-")) {
            addons.insert("table");
        }
    }
    std::string joined;
    for (const auto& addon : addons) {
        if (!joined.empty())
            joined.push_back(',');
        joined.append(addon);
    }
    return joined;
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
                                           ? resolveProcAddress<SetDefaultDirectories>(
                                                 kernel, "SetDefaultDllDirectories")
                                           : nullptr;
    const auto addDirectory =
        kernel ? resolveProcAddress<AddDirectory>(kernel, "AddDllDirectory")
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

// RAII owner of the Rust `fcitx5-engine-core` context/composition/revision
// ledger (E2 cutover: the ledger is the single authoritative owner; the C++
// `nextCompositionId`/`compositions`/`revisions` maps are deleted).
class EngineLedger final {
  public:
    EngineLedger() : handle_(fcitx5_engine_core_ledger_new()) {}
    ~EngineLedger() { fcitx5_engine_core_ledger_free(handle_); }
    EngineLedger(const EngineLedger&) = delete;
    EngineLedger& operator=(const EngineLedger&) = delete;
    [[nodiscard]] void* get() const noexcept { return handle_; }

  private:
    void* handle_;
};

FcitxEngineContextKeyC toLedgerKey(const ClientContextKey& key) {
    return FcitxEngineContextKeyC{key.processId, key.connectionId, key.contextId};
}

FcitxEngineCaretC toLedgerCaret(const protocol::CaretRect& caret) {
    return FcitxEngineCaretC{caret.valid, caret.left, caret.top, caret.right, caret.bottom,
                             caret.dpi};
}

protocol::CaretRect fromLedgerCaret(const FcitxEngineCaretC& caret) {
    return protocol::CaretRect{caret.valid != 0, caret.left, caret.top, caret.right,
                               caret.bottom, caret.dpi};
}

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
    bool applySurroundingText(const protocol::KeyRequest& request,
                              const FcitxEngineKeyDecisionC& decision) {
        // E3: the surrounding-text decision is Rust-owned; this adapter only
        // executes the returned action against Fcitx.
        if (decision.surroundingAction == FCITX_ENGINE_CORE_SURROUNDING_TEXT_ACTION_SET) {
            surroundingText().setText(request.surroundingTextUtf8,
                                      request.surroundingCursor,
                                      request.surroundingAnchor);
            surroundingTextValid_ = true;
        } else {
            surroundingText().invalidate();
            surroundingTextValid_ = false;
        }
        return decision.surroundingUpdate != 0;
    }

    [[nodiscard]] bool hasSurroundingText() const noexcept {
        return surroundingTextValid_;
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

bool warmupHasNoUserState(EngineInputContext& context) {
    if (!context.takeCommit().empty() ||
        context.takeDeleteSurroundingText() ||
        context.takeForwardKey()) {
        return false;
    }
    if (const auto [preedit, caret] = readPreedit(context); !preedit.empty() || caret != 0) {
        return false;
    }
    const auto candidateList = context.inputPanel().candidateList();
    return !candidateList || candidateList->empty();
}

// E5-1: snapshot/status canonicalization is Rust-owned. The adapter maps the
// Rust content-locale code back to the display string and fetches the
// canonical short label through the C ABI.
std::string contentLocaleToString(int locale) {
    switch (locale) {
    case FCITX_ENGINE_CORE_CONTENT_LOCALE_ZH_CN:
        return "zh-CN";
    case FCITX_ENGINE_CORE_CONTENT_LOCALE_JA_JP:
        return "ja-JP";
    case FCITX_ENGINE_CORE_CONTENT_LOCALE_KO_KR:
        return "ko-KR";
    case FCITX_ENGINE_CORE_CONTENT_LOCALE_EN_US:
        return "en-US";
    default:
        return {};
    }
}

std::string shortLabelFromRust(std::string_view text) {
    std::uint8_t buffer[16]{};
    const auto written = fcitx5_engine_core_status_short_label(
        reinterpret_cast<const std::uint8_t*>(text.data()), text.size(), buffer,
        sizeof(buffer));
    if (written > sizeof(buffer))
        return {};
    return std::string(reinterpret_cast<const char*>(buffer), written);
}


// E5-3: canonical snapshot blob serialization. The blob format is
// Rust-authoritative (`rust/engine-core/src/snapshot.rs`); these helpers only
// marshal `RuntimeResult` in/out of that byte format for the pending store.
void writeU32(std::vector<std::uint8_t>& out, std::uint32_t value) {
    out.push_back(static_cast<std::uint8_t>(value));
    out.push_back(static_cast<std::uint8_t>(value >> 8U));
    out.push_back(static_cast<std::uint8_t>(value >> 16U));
    out.push_back(static_cast<std::uint8_t>(value >> 24U));
}

void writeI32(std::vector<std::uint8_t>& out, std::int32_t value) {
    writeU32(out, static_cast<std::uint32_t>(value));
}

void writeU64(std::vector<std::uint8_t>& out, std::uint64_t value) {
    for (int index = 0; index < 8; ++index) {
        out.push_back(static_cast<std::uint8_t>(value >> (8 * index)));
    }
}

void writeBytes(std::vector<std::uint8_t>& out, std::string_view text) {
    writeU32(out, static_cast<std::uint32_t>(text.size()));
    out.insert(out.end(), text.begin(), text.end());
}

std::vector<std::uint8_t> serializeSnapshot(const RuntimeResult& result) {
    std::vector<std::uint8_t> out;
    out.push_back(result.handled ? 1U : 0U);
    writeU32(out, result.preeditCaretUtf8);
    writeU64(out, result.compositionId);
    writeU64(out, result.revision);
    writeU32(out, result.selectedCandidate);
    writeU32(out, result.candidatePage);
    writeU32(out, result.candidateTotal);
    out.push_back(result.candidateVisibility);
    writeU32(out, result.candidatePageSize);
    out.push_back(result.candidateBulk ? 1U : 0U);
    out.push_back(result.candidateEnd ? 1U : 0U);
    out.push_back(result.deleteSurroundingText ? 1U : 0U);
    writeI32(out, result.deleteSurroundingOffset);
    writeU32(out, result.deleteSurroundingSize);
    out.push_back(result.forwardKey ? 1U : 0U);
    writeU32(out, result.forwardKeySym);
    writeU32(out, result.forwardKeyStates);
    writeI32(out, result.forwardKeyCode);
    out.push_back(result.forwardKeyRelease ? 1U : 0U);
    out.push_back(result.caret.valid ? 1U : 0U);
    writeI32(out, result.caret.left);
    writeI32(out, result.caret.top);
    writeI32(out, result.caret.right);
    writeI32(out, result.caret.bottom);
    writeU32(out, result.caret.dpi);
    out.push_back(result.popupAllowed ? 1U : 0U);
    writeBytes(out, result.commitUtf8);
    writeBytes(out, result.preeditUtf8);
    writeBytes(out, result.contentLocaleUtf8);
    writeU32(out, static_cast<std::uint32_t>(result.candidates.size()));
    for (const auto& candidate : result.candidates) {
        writeU64(out, candidate.id);
        writeBytes(out, candidate.labelUtf8);
        writeBytes(out, candidate.textUtf8);
        writeBytes(out, candidate.commentUtf8);
    }
    return out;
}

struct BlobReader {
    const std::uint8_t* data;
    std::size_t size;
    std::size_t offset{};

    bool take(std::size_t count, const std::uint8_t*& pointer) {
        if (count > size - offset)
            return false;
        pointer = data + offset;
        offset += count;
        return true;
    }
    bool u8(std::uint8_t& value) {
        const std::uint8_t* pointer = nullptr;
        if (!take(1, pointer))
            return false;
        value = *pointer;
        return true;
    }
    bool u32(std::uint32_t& value) {
        const std::uint8_t* pointer = nullptr;
        if (!take(4, pointer))
            return false;
        value = static_cast<std::uint32_t>(pointer[0]) |
                (static_cast<std::uint32_t>(pointer[1]) << 8U) |
                (static_cast<std::uint32_t>(pointer[2]) << 16U) |
                (static_cast<std::uint32_t>(pointer[3]) << 24U);
        return true;
    }
    bool i32(std::int32_t& value) {
        std::uint32_t raw = 0;
        if (!u32(raw))
            return false;
        value = static_cast<std::int32_t>(raw);
        return true;
    }
    bool u64(std::uint64_t& value) {
        const std::uint8_t* pointer = nullptr;
        if (!take(8, pointer))
            return false;
        value = 0;
        for (int index = 0; index < 8; ++index)
            value |= static_cast<std::uint64_t>(pointer[index]) << (8 * index);
        return true;
    }
    bool bytes(std::string& text) {
        std::uint32_t length = 0;
        if (!u32(length))
            return false;
        const std::uint8_t* pointer = nullptr;
        if (!take(length, pointer))
            return false;
        text.assign(reinterpret_cast<const char*>(pointer), length);
        return true;
    }
};

bool deserializeSnapshot(const std::uint8_t* data, std::size_t size, RuntimeResult& result) {
    BlobReader reader{data, size};
    std::uint8_t value8 = 0;
    std::uint32_t value32 = 0;
    if (!reader.u8(value8))
        return false;
    result.handled = value8 != 0;
    if (!reader.u32(result.preeditCaretUtf8) || !reader.u64(result.compositionId) ||
        !reader.u64(result.revision) || !reader.u32(result.selectedCandidate) ||
        !reader.u32(result.candidatePage) || !reader.u32(result.candidateTotal) ||
        !reader.u8(result.candidateVisibility) || !reader.u32(result.candidatePageSize))
        return false;
    if (!reader.u8(value8))
        return false;
    result.candidateBulk = value8 != 0;
    if (!reader.u8(value8))
        return false;
    result.candidateEnd = value8 != 0;
    if (!reader.u8(value8))
        return false;
    result.deleteSurroundingText = value8 != 0;
    if (!reader.i32(result.deleteSurroundingOffset) || !reader.u32(result.deleteSurroundingSize))
        return false;
    if (!reader.u8(value8))
        return false;
    result.forwardKey = value8 != 0;
    if (!reader.u32(result.forwardKeySym) || !reader.u32(result.forwardKeyStates) ||
        !reader.i32(result.forwardKeyCode))
        return false;
    if (!reader.u8(value8))
        return false;
    result.forwardKeyRelease = value8 != 0;
    if (!reader.u8(value8))
        return false;
    result.caret.valid = value8 != 0;
    if (!reader.i32(result.caret.left) || !reader.i32(result.caret.top) ||
        !reader.i32(result.caret.right) || !reader.i32(result.caret.bottom) ||
        !reader.u32(result.caret.dpi))
        return false;
    if (!reader.u8(value8))
        return false;
    result.popupAllowed = value8 != 0;
    if (!reader.bytes(result.commitUtf8) || !reader.bytes(result.preeditUtf8) ||
        !reader.bytes(result.contentLocaleUtf8) || !reader.u32(value32))
        return false;
    if (value32 > protocol::kMaxCandidates)
        return false;
    result.candidates.clear();
    result.candidates.reserve(value32);
    for (std::uint32_t index = 0; index < value32; ++index) {
        protocol::CandidateRecord record;
        if (!reader.u64(record.id) || !reader.bytes(record.labelUtf8) ||
            !reader.bytes(record.textUtf8) || !reader.bytes(record.commentUtf8))
            return false;
        result.candidates.push_back(std::move(record));
    }
    return reader.offset == reader.size;
}

} // namespace

class FcitxRuntime::Impl final {
  public:
    std::unique_ptr<Instance> instance;
    std::unordered_map<ClientContextKey, std::unique_ptr<EngineInputContext>, KeyHash> contexts;
    std::unique_ptr<EngineInputContext> warmupContext;
    EngineInputContext* focused{};
    // Rust-authoritative context/composition/revision ledger (E2 cutover).
    // The ledger also owns the per-context product state maps
    // (`carets`/`popupAllowed`/`selectedOverride`/`inputMethodOverridden`).
    EngineLedger ledger;
    // Immutable snapshot of config.toml, loaded once at startup. The input
    // hot path (processKey) reads this in memory instead of reopening and
    // re-parsing the file on every keystroke.
    EngineConfig config;

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
        std::string selected;
        if (!previousDefault.empty() && previousDefault != "keyboard-us" &&
            contains(previousDefault)) {
            selected = previousDefault;
        }
        const std::string configuredDefault = config.defaultInputMethod.value_or("");
        if (selected.empty() && !configuredDefault.empty() && contains(configuredDefault)) {
            selected = configuredDefault;
        }
        for (auto fallback = config.enabled.begin();
             selected.empty() && fallback != config.enabled.end(); ++fallback) {
            if (contains(*fallback))
                selected = *fallback;
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
        const FcitxEngineContextKeyC ledgerKey = toLedgerKey(key);
        (void)fcitx5_engine_core_set_input_method_overridden(ledger.get(), &ledgerKey, 1);
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
            const FcitxEngineContextKeyC ledgerKey = toLedgerKey(key);
            (void)fcitx5_engine_core_set_input_method_overridden(ledger.get(), &ledgerKey, 1);
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
        const FcitxEngineContextKeyC ledgerKey = toLedgerKey(iterator->first);
        fcitx5_engine_core_ledger_forget(ledger.get(), &ledgerKey);
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
        const FcitxEngineContextKeyC ledgerKey = toLedgerKey(key);
        int popupAllowed = 0;
        if (fcitx5_engine_core_popup_allowed(ledger.get(), &ledgerKey, &popupAllowed))
            output.popupAllowed = popupAllowed != 0;
        output.contentLocaleUtf8 = contentLocaleToString(
            fcitx5_engine_core_content_locale_for_input_method(
                instance
                    ? instance->inputMethod(&context).c_str()
                    : ""));
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
        std::uint64_t composition = 0;
        std::uint64_t revision = 0;
        (void)fcitx5_engine_core_ledger_end_result(
            ledger.get(), &ledgerKey,
            (!output.preeditUtf8.empty() || hasCandidates) ? 1 : 0,
            &composition, &revision);
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
            std::uint32_t overrideValue = 0;
            if (fcitx5_engine_core_selected_override(ledger.get(), &ledgerKey,
                                                     &overrideValue)) {
                cursor = static_cast<int>(overrideValue);
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
                    std::uint32_t scrollOffset = 0;
                    if (fcitx5_engine_core_scroll_label_offset(
                            config.orientation == config::Orientation::vertical ? 1 : 0,
                            static_cast<std::uint32_t>((std::max)(0, cursor)),
                            static_cast<std::uint32_t>(index),
                            static_cast<std::uint32_t>(dimension),
                            static_cast<std::uint32_t>(size), &scrollOffset)) {
                        label = std::to_string(scrollOffset + 1U);
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
        output.revision = revision;
        FcitxEngineCaretC caretC{};
        if (fcitx5_engine_core_caret(ledger.get(), &ledgerKey, &caretC))
            output.caret = fromLedgerCaret(caretC);
        // E5-2: the canonical snapshot payload budgets are Rust-validated;
        // fail closed if the snapshot exceeds the wire limits.
        FcitxEngineSnapshotC snapshot{};
        snapshot.handled = output.handled ? 1U : 0U;
        snapshot.commitUtf8Len = output.commitUtf8.size();
        snapshot.preeditUtf8Len = output.preeditUtf8.size();
        snapshot.preeditCaretUtf8 = output.preeditCaretUtf8;
        snapshot.compositionId = output.compositionId;
        snapshot.revision = output.revision;
        snapshot.candidateCount = static_cast<std::uint32_t>(output.candidates.size());
        std::size_t labelMax = 0;
        std::size_t textMax = 0;
        std::size_t commentMax = 0;
        for (const auto& candidate : output.candidates) {
            labelMax = (std::max)(labelMax, candidate.labelUtf8.size());
            textMax = (std::max)(textMax, candidate.textUtf8.size());
            commentMax = (std::max)(commentMax, candidate.commentUtf8.size());
        }
        snapshot.candidateLabelLenMax = labelMax;
        snapshot.candidateTextLenMax = textMax;
        snapshot.candidateCommentLenMax = commentMax;
        snapshot.contentLocaleUtf8Len = output.contentLocaleUtf8.size();
        snapshot.selectedCandidate = output.selectedCandidate;
        snapshot.candidatePage = output.candidatePage;
        snapshot.candidateTotal = output.candidateTotal;
        snapshot.candidateVisibility = output.candidateVisibility;
        snapshot.candidatePageSize = output.candidatePageSize;
        snapshot.candidateBulk = output.candidateBulk ? 1U : 0U;
        snapshot.candidateEnd = output.candidateEnd ? 1U : 0U;
        snapshot.deleteSurroundingText = output.deleteSurroundingText ? 1U : 0U;
        snapshot.deleteSurroundingOffset = output.deleteSurroundingOffset;
        snapshot.deleteSurroundingSize = output.deleteSurroundingSize;
        snapshot.forwardKey = output.forwardKey ? 1U : 0U;
        snapshot.forwardKeySym = output.forwardKeySym;
        snapshot.forwardKeyStates = output.forwardKeyStates;
        snapshot.forwardKeyCode = output.forwardKeyCode;
        snapshot.forwardKeyRelease = output.forwardKeyRelease ? 1U : 0U;
        snapshot.caretValid = output.caret.valid ? 1U : 0U;
        snapshot.popupAllowed = output.popupAllowed ? 1U : 0U;
        if (fcitx5_engine_core_validate_snapshot(&snapshot) == 0) {
            throw std::invalid_argument("invalid engine snapshot");
        }
        return output;
    }
};

FcitxRuntime::FcitxRuntime() : impl_(std::make_unique<Impl>()) {}
FcitxRuntime::~FcitxRuntime() = default;

bool FcitxRuntime::initialize(bool safeMode) {
    try {
        if (!setupEnvironment())
            return false;
        impl_->config = readEngineConfig();
        std::string enabledAddons;
        std::vector<std::string> arguments;
        if (safeMode) {
            enabledAddons = "windowskeyboard,pinyin,punctuation";
        } else {
            // Do not let the set of installed optional addons decide the
            // product startup surface. A clean Windows profile defaults to
            // pinyin-only, so normal startup explicitly enables the adapter
            // and addons needed by the configured input methods/tests instead
            // of loading every bundled on-demand addon (for example Rime,
            // table, or spell) before the first TSF client can type.
            enabledAddons = startupAddonList(impl_->config);
        }
        arguments = {"fcitx5-engine", "--disable=all", "--enable=" + enabledAddons};
        std::vector<char*> argv;
        argv.reserve(arguments.size());
        for (auto& argument : arguments)
            argv.push_back(argument.data());
        impl_->instance =
            std::make_unique<Instance>(static_cast<int>(argv.size()), argv.data());
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
            if (!warmupHasNoUserState(*impl_->warmupContext))
                return false;
            impl_->warmupContext->reset();
        }
        impl_->instance->setCurrentInputMethod(impl_->warmupContext.get(), selected, true);
        impl_->dispatchPendingEvents();
        if (!warmupHasNoUserState(*impl_->warmupContext))
            return false;
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
    const FcitxEngineContextKeyC ledgerKey = toLedgerKey(key);
    if (fcitx5_engine_core_ledger_begin_key(
            impl_->ledger.get(), &ledgerKey, request.metadata.revision,
            request.metadata.compositionId) != FCITX_ENGINE_CORE_OK) {
        throw std::invalid_argument("stale input context state");
    }
    auto& context = impl_->contextFor(key);
    (void)fcitx5_engine_core_set_popup_allowed(impl_->ledger.get(), &ledgerKey,
                                               request.popupAllowed ? 1 : 0);
    if (impl_->focused != &context) {
        if (impl_->focused && impl_->focused->hasFocus())
            impl_->focused->focusOut();
        context.focusIn();
        impl_->focused = &context;
        impl_->dispatchPendingEvents();
    }
    // E3 event-shape consolidation: the unified Event→Action decision is
    // Rust-owned (`fcitx5_engine_core_handle_key_event`); the adapter only
    // flattens Fcitx facts and executes the returned decision.
    KeyEvent event(&context, keyFromRequest(request),
                   (request.keyFlags & protocol::kKeyFlagRelease) != 0);
    const KeySym keySym = event.key().sym();
    const auto& group = impl_->instance->inputMethodManager().currentGroup();
    const std::string& requestIm = request.inputMethodUtf8;
    const std::string& defaultIm = group.defaultInputMethod();
    const bool hasRequestIm = !requestIm.empty();
    const bool requestImValid =
        hasRequestIm && impl_->instance->inputMethodManager().entry(requestIm) != nullptr;
    const bool defaultImValid =
        impl_->instance->inputMethodManager().entry(defaultIm) != nullptr;
    const std::string& currentIm = impl_->instance->inputMethod(&context);
    int inputMethodOverridden = 0;
    const bool hasImOverride =
        fcitx5_engine_core_input_method_overridden(impl_->ledger.get(), &ledgerKey,
                                                   &inputMethodOverridden) != 0;
    FcitxEngineKeyEventC keyEvent{};
    keyEvent.keySym = static_cast<std::uint32_t>(keySym);
    keyEvent.keyFlags = request.keyFlags;
    keyEvent.isRelease = event.isRelease() ? 1U : 0U;
    keyEvent.hotkeyToggle =
        impl_->config.hotkeyToggle ? impl_->config.hotkeyToggle->c_str() : nullptr;
    keyEvent.hotkeyNext =
        impl_->config.hotkeyNext ? impl_->config.hotkeyNext->c_str() : nullptr;
    keyEvent.surroundingTextValid = request.surroundingTextValid ? 1U : 0U;
    keyEvent.currentSurroundingValid = context.hasSurroundingText() ? 1U : 0U;
    keyEvent.hasRequestIm = hasRequestIm ? 1U : 0U;
    keyEvent.requestImValid = requestImValid ? 1U : 0U;
    keyEvent.defaultImValid = defaultImValid ? 1U : 0U;
    keyEvent.defaultImNonempty = !defaultIm.empty() ? 1U : 0U;
    keyEvent.currentEqRequest = hasRequestIm && currentIm == requestIm ? 1U : 0U;
    keyEvent.currentEqDefault = currentIm == defaultIm ? 1U : 0U;
    keyEvent.imOverridden = hasImOverride && inputMethodOverridden != 0 ? 1U : 0U;
    if (const auto list = context.inputPanel().candidateList();
        list && !list->empty()) {
        const auto* bulk = list->toBulk();
        const auto* bulkCursor = list->toBulkCursor();
        auto* pageable = list->toPageable();
        keyEvent.hasCandidates = 1U;
        keyEvent.view.count =
            bulk && bulk->totalSize() >= 0 ? bulk->totalSize() : list->size();
        keyEvent.view.listSize = static_cast<std::int32_t>(list->size());
        keyEvent.view.cursor = static_cast<std::int32_t>(list->cursorIndex());
        keyEvent.view.bulkCursor =
            bulkCursor ? static_cast<std::int32_t>(bulkCursor->globalCursorIndex()) : -1;
        keyEvent.view.hasBulkCursor = bulkCursor ? 1U : 0U;
        keyEvent.view.hasBulk = bulk && bulk->totalSize() >= 0 ? 1U : 0U;
        keyEvent.view.pageable = pageable ? 1U : 0U;
        keyEvent.view.hasPrev = pageable && pageable->hasPrev() ? 1U : 0U;
        keyEvent.view.hasNext = pageable && pageable->hasNext() ? 1U : 0U;
        keyEvent.config.scrollMode = impl_->config.scrollMode ? 1U : 0U;
        keyEvent.config.vertical =
            impl_->config.orientation == config::Orientation::vertical ? 1U : 0U;
        keyEvent.config.candidatePageSize =
            impl_->config.candidatePageSize
                ? static_cast<std::int32_t>(*impl_->config.candidatePageSize)
                : -1;
        std::uint32_t overrideValue = 0;
        if (fcitx5_engine_core_selected_override(impl_->ledger.get(), &ledgerKey,
                                                 &overrideValue)) {
            keyEvent.hasOverride = 1U;
            keyEvent.overrideValue = overrideValue;
        }
    }
    FcitxEngineKeyDecisionC decision{};
    if (fcitx5_engine_core_handle_key_event(&keyEvent, &decision) != FCITX_ENGINE_CORE_OK) {
        // Fail closed: forward the key to Fcitx without any product action.
        context.keyEvent(event);
        const FcitxEngineCaretC failCaret = toLedgerCaret(request.caret);
        (void)fcitx5_engine_core_set_caret(impl_->ledger.get(), &ledgerKey, &failCaret);
        return impl_->collectResult(key, context, event.accepted());
    }
    // Execute the surrounding-text action.
    if (context.applySurroundingText(request, decision)) {
        context.updateSurroundingText();
    }
    // Execute the input-method selection.
    if (decision.imSelection != FCITX_ENGINE_CORE_IM_SELECTION_NONE) {
        const std::string& target =
            decision.imSelection == FCITX_ENGINE_CORE_IM_SELECTION_REQUEST ? requestIm
                                                                           : defaultIm;
        (void)impl_->instance->inputMethodEngine(target);
        impl_->instance->setCurrentInputMethod(&context, target, true);
        impl_->dispatchPendingEvents();
    }
    // Main path: input-method switch hotkey.
    if (decision.imSwitch == FCITX_ENGINE_CORE_IM_ACTION_TOGGLE) {
        impl_->toggleInputMethod(impl_->config, key, context);
        event.filterAndAccept();
        return impl_->collectResult(key, context, true);
    }
    if (decision.imSwitch == FCITX_ENGINE_CORE_IM_ACTION_NEXT) {
        impl_->nextInputMethod(impl_->config, key, context);
        event.filterAndAccept();
        return impl_->collectResult(key, context, true);
    }
    // Main path: candidate navigation.
    if (decision.candidateConsume) {
        const auto list = context.inputPanel().candidateList();
        if (list) {
            const auto* bulk = list->toBulk();
            auto* pageable = list->toPageable();
            const auto selectCandidate = [&](std::uint32_t index) {
                const auto& candidate =
                    bulk && bulk->totalSize() >= 0
                        ? bulk->candidateFromAll(static_cast<int>(index))
                        : list->candidate(static_cast<int>(index));
                candidate.select(&context);
            };
            switch (decision.candidateAction) {
            case FCITX_ENGINE_CORE_CANDIDATE_ACTION_SELECT_AND_CLEAR:
                selectCandidate(decision.candidateValue);
                (void)fcitx5_engine_core_clear_selected_override(impl_->ledger.get(),
                                                                 &ledgerKey);
                break;
            case FCITX_ENGINE_CORE_CANDIDATE_ACTION_SET_OVERRIDE:
                (void)fcitx5_engine_core_set_selected_override(impl_->ledger.get(),
                                                               &ledgerKey,
                                                               decision.candidateValue);
                break;
            case FCITX_ENGINE_CORE_CANDIDATE_ACTION_PAGE_NEXT_AND_SET_OVERRIDE:
                if (pageable) {
                    pageable->next();
                }
                (void)fcitx5_engine_core_set_selected_override(impl_->ledger.get(),
                                                               &ledgerKey,
                                                               decision.candidateValue);
                break;
            case FCITX_ENGINE_CORE_CANDIDATE_ACTION_PAGE_PREV_AND_SET_OVERRIDE:
                if (pageable) {
                    pageable->prev();
                }
                (void)fcitx5_engine_core_set_selected_override(impl_->ledger.get(),
                                                               &ledgerKey,
                                                               decision.candidateValue);
                break;
            default:
                break;
            }
        }
        event.filterAndAccept();
        return impl_->collectResult(key, context, true);
    }
    // Forward the key to Fcitx.
    if (decision.clearOverride) {
        (void)fcitx5_engine_core_clear_selected_override(impl_->ledger.get(), &ledgerKey);
    }
    context.keyEvent(event);
    const FcitxEngineCaretC caretC = toLedgerCaret(request.caret);
    (void)fcitx5_engine_core_set_caret(impl_->ledger.get(), &ledgerKey, &caretC);
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
    if (found == impl_->contexts.end()) {
        throw std::invalid_argument("stale candidate selection state");
    }
    const FcitxEngineContextKeyC ledgerKey = toLedgerKey(found->first);
    if (fcitx5_engine_core_ledger_select_candidate(
            impl_->ledger.get(), &ledgerKey, request.metadata.revision,
            request.metadata.compositionId, request.candidateId) !=
        FCITX_ENGINE_CORE_OK) {
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
    (void)fcitx5_engine_core_clear_selected_override(impl_->ledger.get(), &ledgerKey);
    RuntimeResult output = impl_->collectResult(found->first, context, true);
    // E5-3: the pending snapshot store is Rust-owned.
    const auto blob = serializeSnapshot(output);
    (void)fcitx5_engine_core_snapshot_store_put(impl_->ledger.get(), &ledgerKey,
                                                output.revision, blob.data(), blob.size());
    return output;
}

RuntimeResult FcitxRuntime::takePendingState(
    const ClientContextKey& key, const protocol::StateRequest& request) {
    const FcitxEngineContextKeyC ledgerKey = toLedgerKey(key);
    const std::size_t required =
        fcitx5_engine_core_snapshot_store_required_size(impl_->ledger.get(), &ledgerKey);
    if (required == 0)
        throw std::invalid_argument("pending state is unavailable");
    std::vector<std::uint8_t> blob(required);
    std::size_t blobLength = 0;
    if (fcitx5_engine_core_snapshot_store_take(
            impl_->ledger.get(), &ledgerKey, request.metadata.revision, blob.data(),
            blob.size(), &blobLength) == 0) {
        throw std::invalid_argument("pending state is unavailable");
    }
    RuntimeResult output;
    if (!deserializeSnapshot(blob.data(), blobLength, output))
        throw std::invalid_argument("pending state is unavailable");
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
        output.shortLabel = shortLabelFromRust(output.name);
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
    output.shortLabel = shortLabelFromRust(display);
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
