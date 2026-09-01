#pragma once

#include "protocol_ffi.h"

#include <fcitx-utils/key.h>

namespace fcitx::windows::engine {

[[nodiscard]] fcitx::Key keyFromRequest(const FcitxKeyRequestC& request);

} // namespace fcitx::windows::engine
