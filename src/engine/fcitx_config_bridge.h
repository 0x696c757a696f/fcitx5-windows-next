#pragma once

#include <optional>
#include <string>
#include <string_view>

namespace fcitx {
class Instance;
}

namespace fcitx::windows::engine {

// Reads one Fcitx configuration for Settings. `uri` is one of
//   fcitx://config/global
//   fcitx://config/addon/<addon>[/<sub config>]
//   fcitx://config/inputmethod/<input method>
//   fcitx://config/addons        (every addon and whether it is configurable)
//   fcitx://config/inputmethods  (every input method and whether it is configurable)
// and the result is JSON. A configuration is
//   {"description": <tree>, "value": <tree>}
// where a tree is {"v": "<value>", "c": [["<name>", <tree>], ...]}, the
// RawConfig of Configuration::dumpDescription and Configuration::save in
// order. Settings merges the two the way fcitx5-macos does.
std::optional<std::string> fcitxConfigGet(Instance& instance, std::string_view uri);

// Applies `value` to `uri` and saves it. `value` holds one option per line as
// "<path>\t<value>", where the path is '/'-separated and the value escapes
// '\\', '\t' and '\n' with a backslash.
bool fcitxConfigSet(Instance& instance, std::string_view uri, std::string_view value);

} // namespace fcitx::windows::engine
