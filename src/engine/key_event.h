#pragma once

#include "protocol.h"

#include <fcitx-utils/key.h>

namespace fcitx::windows::engine {

[[nodiscard]] fcitx::Key keyFromRequest(const protocol::KeyRequest& request);

} // namespace fcitx::windows::engine
