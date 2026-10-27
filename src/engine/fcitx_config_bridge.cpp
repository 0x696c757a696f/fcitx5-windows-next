#include "fcitx_config_bridge.h"

#include <fcitx-config/configuration.h>
#include <fcitx-config/rawconfig.h>
#include <fcitx/addoninfo.h>
#include <fcitx/addoninstance.h>
#include <fcitx/addonmanager.h>
#include <fcitx/globalconfig.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/inputmethodentry.h>
#include <fcitx/inputmethodmanager.h>
#include <fcitx/instance.h>

#include <algorithm>
#include <cstdio>
#include <vector>

namespace fcitx::windows::engine {
namespace {

constexpr std::string_view kGlobalUri = "fcitx://config/global";
constexpr std::string_view kAddonsUri = "fcitx://config/addons";
constexpr std::string_view kInputMethodsUri = "fcitx://config/inputmethods";
constexpr std::string_view kAddonPrefix = "fcitx://config/addon/";
constexpr std::string_view kInputMethodPrefix = "fcitx://config/inputmethod/";

void appendJsonString(std::string& out, std::string_view value) {
    out.push_back('"');
    for (const char character : value) {
        const auto byte = static_cast<unsigned char>(character);
        if (character == '"' || character == '\\') {
            out.push_back('\\');
            out.push_back(character);
        } else if (byte < 0x20U) {
            char escaped[8]{};
            std::snprintf(escaped, sizeof(escaped), "\\u%04x", static_cast<unsigned>(byte));
            out += escaped;
        } else {
            out.push_back(character);
        }
    }
    out.push_back('"');
}

void appendTree(std::string& out, const RawConfig& config) {
    out += "{\"v\":";
    appendJsonString(out, config.value());
    out += ",\"c\":[";
    bool first = true;
    for (const auto& name : config.subItems()) {
        const auto child = config.get(name);
        if (!child)
            continue;
        if (!first)
            out.push_back(',');
        first = false;
        out.push_back('[');
        appendJsonString(out, name);
        out.push_back(',');
        appendTree(out, *child);
        out.push_back(']');
    }
    out += "]}";
}

std::string configurationJson(const Configuration& configuration) {
    RawConfig description;
    configuration.dumpDescription(description);
    RawConfig value;
    configuration.save(value);
    std::string out = "{\"description\":";
    appendTree(out, description);
    out += ",\"value\":";
    appendTree(out, value);
    out.push_back('}');
    return out;
}

// "<addon>[/<sub config>]" after the addon prefix.
std::pair<std::string, std::string> splitAddonUri(std::string_view uri) {
    const auto rest = uri.substr(kAddonPrefix.size());
    const auto slash = rest.find('/');
    if (slash == std::string_view::npos)
        return {std::string(rest), {}};
    return {std::string(rest.substr(0, slash)), std::string(rest.substr(slash + 1))};
}

// The running addon `uri` names, when it has options. An addon that is not
// running is not started just to show or change its options.
AddonInstance* configurableAddon(Instance& instance, std::string_view uri) {
    const auto name = splitAddonUri(uri).first;
    const auto* info = instance.addonManager().addonInfo(name);
    if (!info || !info->isConfigurable())
        return nullptr;
    return instance.addonManager().lookupAddon(name);
}

std::string addonsJson(Instance& instance) {
    auto& addons = instance.addonManager();
    std::vector<const AddonInfo*> infos;
    for (const auto category : {AddonCategory::InputMethod, AddonCategory::Frontend,
                                AddonCategory::Loader, AddonCategory::Module, AddonCategory::UI}) {
        for (const auto& name : addons.addonNames(category)) {
            if (const auto* info = addons.addonInfo(name); info && info->isValid())
                infos.push_back(info);
        }
    }
    std::sort(infos.begin(), infos.end(), [](const AddonInfo* left, const AddonInfo* right) {
        return left->uniqueName() < right->uniqueName();
    });
    std::string out = "[";
    bool first = true;
    for (const auto* info : infos) {
        if (!first)
            out.push_back(',');
        first = false;
        out += "{\"id\":";
        appendJsonString(out, info->uniqueName());
        out += ",\"name\":";
        appendJsonString(out, info->name().match());
        out += ",\"category\":";
        appendJsonString(out, AddonCategoryToString(info->category()));
        out += ",\"configurable\":";
        out += info->isConfigurable() ? "true" : "false";
        out += ",\"loaded\":";
        out += addons.lookupAddon(info->uniqueName()) ? "true" : "false";
        out.push_back('}');
    }
    out.push_back(']');
    return out;
}

std::string inputMethodsJson(Instance& instance) {
    std::string out = "[";
    bool first = true;
    instance.inputMethodManager().foreachEntries([&](const InputMethodEntry& entry) {
        if (!first)
            out.push_back(',');
        first = false;
        out += "{\"id\":";
        appendJsonString(out, entry.uniqueName());
        out += ",\"name\":";
        appendJsonString(out, entry.name());
        out += ",\"native_name\":";
        appendJsonString(out, entry.nativeName());
        out += ",\"addon\":";
        appendJsonString(out, entry.addon());
        out += ",\"configurable\":";
        out += entry.isConfigurable() ? "true" : "false";
        out.push_back('}');
        return true;
    });
    out.push_back(']');
    return out;
}

// Parses "<path>\t<escaped value>" lines into a RawConfig.
std::optional<RawConfig> parseValueLines(std::string_view text) {
    RawConfig config;
    while (!text.empty()) {
        const auto end = text.find('\n');
        const auto line = text.substr(0, end);
        text = end == std::string_view::npos ? std::string_view{} : text.substr(end + 1);
        if (line.empty())
            continue;
        const auto tab = line.find('\t');
        if (tab == std::string_view::npos || tab == 0)
            return std::nullopt;
        std::string value;
        const auto escaped = line.substr(tab + 1);
        for (std::size_t index = 0; index < escaped.size(); ++index) {
            if (escaped[index] != '\\') {
                value.push_back(escaped[index]);
                continue;
            }
            if (++index >= escaped.size())
                return std::nullopt;
            switch (escaped[index]) {
            case '\\':
                value.push_back('\\');
                break;
            case 't':
                value.push_back('\t');
                break;
            case 'n':
                value.push_back('\n');
                break;
            default:
                return std::nullopt;
            }
        }
        config.setValueByPath(std::string(line.substr(0, tab)), std::move(value));
    }
    return config;
}

} // namespace

std::optional<std::string> fcitxConfigGet(Instance& instance, std::string_view uri) {
    if (uri == kGlobalUri)
        return configurationJson(instance.globalConfig().config());
    if (uri == kAddonsUri)
        return addonsJson(instance);
    if (uri == kInputMethodsUri)
        return inputMethodsJson(instance);
    if (uri.starts_with(kAddonPrefix)) {
        auto* addon = configurableAddon(instance, uri);
        if (!addon)
            return std::nullopt;
        const auto sub = splitAddonUri(uri).second;
        const auto* configuration = sub.empty() ? addon->getConfig() : addon->getSubConfig(sub);
        if (!configuration)
            return std::nullopt;
        return configurationJson(*configuration);
    }
    if (uri.starts_with(kInputMethodPrefix)) {
        const std::string id(uri.substr(kInputMethodPrefix.size()));
        const auto* entry = instance.inputMethodManager().entry(id);
        if (!entry || !entry->isConfigurable())
            return std::nullopt;
        auto* engine = instance.inputMethodEngine(id);
        if (!engine)
            return std::nullopt;
        const auto* configuration = engine->getConfigForInputMethod(*entry);
        if (!configuration)
            return std::nullopt;
        return configurationJson(*configuration);
    }
    return std::nullopt;
}

bool fcitxConfigSet(Instance& instance, std::string_view uri, std::string_view value) {
    auto config = parseValueLines(value);
    if (!config)
        return false;
    if (uri == kGlobalUri) {
        auto& global = instance.globalConfig();
        RawConfig previous;
        global.save(previous);
        global.load(*config, true);
        if (!global.safeSave()) {
            // Keep the engine on what is on disk.
            global.load(previous);
            return false;
        }
        instance.reloadConfig();
        return true;
    }
    if (uri.starts_with(kAddonPrefix)) {
        auto* addon = configurableAddon(instance, uri);
        if (!addon)
            return false;
        const auto sub = splitAddonUri(uri).second;
        if (sub.empty())
            addon->setConfig(*config);
        else
            addon->setSubConfig(sub, *config);
        return true;
    }
    if (uri.starts_with(kInputMethodPrefix)) {
        const std::string id(uri.substr(kInputMethodPrefix.size()));
        const auto* entry = instance.inputMethodManager().entry(id);
        if (!entry || !entry->isConfigurable())
            return false;
        auto* engine = instance.inputMethodEngine(id);
        if (!engine)
            return false;
        // A table describes and saves its options inside [Table] but takes
        // that section itself when it loads them.
        if (entry->addon() == "table") {
            if (const auto table = config->get("Table")) {
                engine->setConfigForInputMethod(*entry, *table);
                return true;
            }
        }
        engine->setConfigForInputMethod(*entry, *config);
        return true;
    }
    return false;
}

} // namespace fcitx::windows::engine
