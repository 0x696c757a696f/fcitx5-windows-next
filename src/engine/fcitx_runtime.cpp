#include "fcitx_runtime.h"

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

#include <algorithm>
#include <filesystem>
#include <memory>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <utility>

namespace fcitx::windows::engine {
namespace {

std::string utf8Path(const std::filesystem::path& path) {
    const auto& native = path.native();
    if (native.empty()) return {};
    const int size = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, native.data(),
                                         static_cast<int>(native.size()), nullptr, 0,
                                         nullptr, nullptr);
    if (size <= 0) return {};
    std::string output(static_cast<std::size_t>(size), '\0');
    return WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, native.data(),
                               static_cast<int>(native.size()), output.data(), size,
                               nullptr, nullptr) == size
               ? output
               : std::string{};
}

bool setupEnvironment() {
    std::wstring modulePath(32'768, L'\0');
    const DWORD size = GetModuleFileNameW(nullptr, modulePath.data(),
                                          static_cast<DWORD>(modulePath.size()));
    if (size == 0 || size >= modulePath.size()) return false;
    modulePath.resize(size);
    const auto root = std::filesystem::path(modulePath).parent_path().parent_path();
    if (!SetDllDirectoryW((root / "bin").c_str())) return false;
    const auto addon = utf8Path(root / "lib" / "fcitx5");
    const auto share = utf8Path(root / "share");
    const auto data = utf8Path(root / "share" / "fcitx5");
    const auto models = utf8Path(root / "lib" / "libime");
    if (addon.empty() || share.empty() || data.empty() || models.empty()) return false;
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
        return (static_cast<std::size_t>(key.processId) * 0x9e3779b1U) ^ high ^ low;
    }
};

class EngineInputContext final : public InputContext {
public:
    explicit EngineInputContext(InputContextManager& manager)
        : InputContext(manager, "tsf") {
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
    const auto vk = request.virtualKey;
    switch (vk) {
    case VK_BACK: return Key(FcitxKey_BackSpace, states);
    case VK_RETURN: return Key(FcitxKey_Return, states);
    case VK_SPACE: return Key(FcitxKey_space, states);
    case VK_ESCAPE: return Key(FcitxKey_Escape, states);
    case VK_LEFT: return Key(FcitxKey_Left, states);
    case VK_RIGHT: return Key(FcitxKey_Right, states);
    case VK_UP: return Key(FcitxKey_Up, states);
    case VK_DOWN: return Key(FcitxKey_Down, states);
    case VK_PRIOR: return Key(FcitxKey_Page_Up, states);
    case VK_NEXT: return Key(FcitxKey_Page_Down, states);
    default: break;
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
    if (cursor < 0) cursor = static_cast<int>(text.size());
    cursor = std::clamp(cursor, 0, static_cast<int>(text.size()));
    return {std::move(text), static_cast<std::uint32_t>(cursor)};
}

} // namespace

class FcitxRuntime::Impl final {
public:
    std::unique_ptr<Instance> instance;
    std::unordered_map<ClientContextKey, std::unique_ptr<EngineInputContext>, KeyHash> contexts;
    EngineInputContext* focused{};
    std::uint64_t nextCompositionId{1};
    std::unordered_map<ClientContextKey, std::uint64_t, KeyHash> revisions;
    std::unordered_map<ClientContextKey, std::uint64_t, KeyHash> compositions;

    void ensureInputMethods() {
        auto& manager = instance->inputMethodManager();
        manager.refresh();
        InputMethodGroup group = manager.currentGroup();
        auto& items = group.inputMethodList();
        auto contains = [&items](const std::string& name) {
            return std::any_of(items.begin(), items.end(), [&](const auto& item) {
                return item.name() == name;
            });
        };
        bool changed = false;
        if (manager.entry("wbx") && !contains("wbx")) {
            items.insert(items.begin(), InputMethodGroupItem("wbx"));
            changed = true;
        }
        if (manager.entry("pinyin") && !contains("pinyin")) {
            items.emplace_back("pinyin");
            changed = true;
        }
        if (manager.entry("pinyin")) {
            group.setDefaultInputMethod("pinyin");
            changed = true;
        }
        if (changed) {
            manager.setGroup(std::move(group));
            manager.save();
        }
    }

    EngineInputContext& contextFor(const ClientContextKey& key) {
        auto found = contexts.find(key);
        if (found != contexts.end()) return *found->second;
        auto context = std::make_unique<EngineInputContext>(instance->inputContextManager());
        auto* result = context.get();
        contexts.emplace(key, std::move(context));
        return *result;
    }
};

FcitxRuntime::FcitxRuntime() : impl_(std::make_unique<Impl>()) {}
FcitxRuntime::~FcitxRuntime() = default;

bool FcitxRuntime::initialize() {
    try {
        if (!setupEnvironment()) return false;
        impl_->instance = std::make_unique<Instance>(0, nullptr);
        impl_->instance->addonManager().registerDefaultLoader(nullptr);
        impl_->instance->initialize();
        impl_->ensureInputMethods();
        if (!impl_->instance->inputMethodManager().entry("pinyin")) return false;
        if (!impl_->instance->inputMethodEngine("pinyin")) return false;
        EngineInputContext warmup(impl_->instance->inputContextManager());
        warmup.focusIn();
        impl_->instance->setCurrentInputMethod(&warmup, "pinyin", true);
        KeyEvent event(&warmup, Key(FcitxKey_n), false);
        warmup.keyEvent(event);
        warmup.reset();
        warmup.focusOut();
        return true;
    } catch (...) {
        impl_->instance.reset();
        return false;
    }
}

::fcitx::EventLoop& FcitxRuntime::eventLoop() { return impl_->instance->eventLoop(); }

RuntimeResult FcitxRuntime::processKey(const ClientContextKey& key,
                                       const protocol::KeyRequest& request) {
    RuntimeResult output;
    const auto currentRevision = impl_->revisions[key];
    const auto currentComposition = impl_->compositions[key];
    if (request.metadata.revision != currentRevision ||
        request.metadata.compositionId != currentComposition) {
        throw std::invalid_argument("stale input context state");
    }
    auto& context = impl_->contextFor(key);
    if (impl_->focused != &context) {
        if (impl_->focused && impl_->focused->hasFocus()) impl_->focused->focusOut();
        context.focusIn();
        impl_->focused = &context;
    }
    if (impl_->instance->inputMethodManager().entry("pinyin") &&
        impl_->instance->inputMethod(&context) != "pinyin") {
        (void)impl_->instance->inputMethodEngine("pinyin");
        impl_->instance->setCurrentInputMethod(&context, "pinyin", true);
    }
    KeyEvent event(&context, keyFromRequest(request), false);
    context.keyEvent(event);
    output.handled = event.accepted();
    output.commitUtf8 = context.takeCommit();
    auto [preedit, caret] = readPreedit(context);
    output.preeditUtf8 = std::move(preedit);
    output.preeditCaretUtf8 = caret;
    auto& composition = impl_->compositions[key];
    if (!output.preeditUtf8.empty() && composition == 0) {
        composition = impl_->nextCompositionId++;
        if (composition == 0) composition = impl_->nextCompositionId++;
    }
    if (output.preeditUtf8.empty()) composition = 0;
    output.compositionId = composition;
    output.revision = ++impl_->revisions[key];
    return output;
}

void FcitxRuntime::forgetProcess(std::uint32_t processId) {
    for (auto iterator = impl_->contexts.begin(); iterator != impl_->contexts.end();) {
        if (iterator->first.processId == processId) {
            if (impl_->focused == iterator->second.get()) impl_->focused = nullptr;
            impl_->revisions.erase(iterator->first);
            impl_->compositions.erase(iterator->first);
            iterator = impl_->contexts.erase(iterator);
        } else {
            ++iterator;
        }
    }
}

} // namespace fcitx::windows::engine
