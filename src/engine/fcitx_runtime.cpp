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
#include <iterator>
#include <memory>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <utility>

namespace fcitx::windows::engine {
namespace {

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
        for (const char* id : {"pinyin", "rime", "wbx"}) {
            if (manager.entry(id) && !contains(id)) {
                items.emplace_back(id);
            }
        }
        std::string selected = previousDefault;
        if (!contains(selected)) {
            selected.clear();
            for (const char* fallback : {"pinyin", "rime", "wbx"}) {
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
            const int size =
                bulk && reportedTotal >= 0
                    ? std::clamp(reportedTotal, 0,
                                 static_cast<int>(protocol::kMaxCandidates))
                    : pageSize;
            output.candidateBulk = bulk != nullptr;
            output.candidateEnd = !bulk || (reportedTotal >= 0 && size >= reportedTotal);
            output.candidates.reserve(static_cast<std::size_t>(size));
            for (int index = 0; index < size; ++index) {
                const auto& word =
                    bulk ? bulk->candidateFromAll(index) : candidateList->candidate(index);
                output.candidates.push_back(protocol::CandidateRecord{
                    (composition << 8U) | static_cast<std::uint64_t>(index + 1),
                    bulk ? std::string{} : candidateList->label(index).toString(),
                    word.text().toString(), word.comment().toString()});
            }
            const int cursor = candidateList->toBulkCursor()
                                   ? candidateList->toBulkCursor()->globalCursorIndex()
                                   : candidateList->cursorIndex();
            if (cursor >= 0 && cursor < size)
                output.selectedCandidate = static_cast<std::uint32_t>(cursor);
            output.candidateTotal = static_cast<std::uint32_t>(size);
            if (bulk && bulk->totalSize() >= 0)
                output.candidateTotal = static_cast<std::uint32_t>(bulk->totalSize());
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
    if (!selected.empty() && impl_->instance->inputMethodManager().entry(selected) &&
        impl_->instance->inputMethod(&context) != selected) {
        (void)impl_->instance->inputMethodEngine(selected);
        impl_->instance->setCurrentInputMethod(&context, selected, true);
    }
    KeyEvent event(&context, keyFromRequest(request), false);
    // Laptop-friendly page keys: '-' / '_' previous page, '=' / '+' next page.
    // Fcitx's default PrevPage/NextPage are Up/Down, which the scroll viewport
    // uses for continuous cursor movement, so route the number-row keys to the
    // pageable candidate list explicitly when candidates are visible.
    if (event.key().isSimple()) {
        const KeySym sym = event.key().sym();
        if (const auto list = context.inputPanel().candidateList();
            list && !list->empty()) {
            if (auto* pageable = list->toPageable(); pageable) {
                if ((sym == FcitxKey_equal || sym == FcitxKey_plus) &&
                    pageable->hasNext()) {
                    pageable->next();
                    event.filter();
                } else if ((sym == FcitxKey_minus || sym == FcitxKey_underscore) &&
                           pageable->hasPrev()) {
                    pageable->prev();
                    event.filter();
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
            iterator = impl_->contexts.erase(iterator);
        } else {
            ++iterator;
        }
    }
}

} // namespace fcitx::windows::engine
