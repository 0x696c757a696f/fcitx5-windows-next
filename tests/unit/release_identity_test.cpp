#include <fcitx5_windows/release_identity.h>
#include <fcitx5_windows/version.h>

#include <windows.h>
#include <objbase.h>

#include <iostream>
#include <set>
#include <string>

namespace {

std::wstring guid_text(const GUID& guid) {
  wchar_t text[40]{};
  if (StringFromGUID2(guid, text, 40) == 0) return {};
  return text;
}

}  // namespace

int main() {
  using namespace fcitx::windows;
  std::set<std::wstring> clsids;
  std::set<std::wstring> profiles;
  std::set<std::wstring> tray_icons;
  std::set<std::wstring> pipes;
  std::set<std::wstring> app_ids;
  std::set<std::wstring> settings_app_ids;
  for (const auto& identity : kReleaseIdentities) {
    if (identity.channel_name.empty() || !identity.service_description || !identity.pipe_prefix ||
        !identity.data_directory || !identity.registry_value || !identity.installer_app_id ||
        !identity.settings_app_user_model_id ||
        !clsids.emplace(guid_text(identity.text_service_clsid)).second ||
        !profiles.emplace(guid_text(identity.language_profile_guid)).second ||
        !tray_icons.emplace(guid_text(identity.notification_icon_guid)).second ||
        !pipes.emplace(identity.pipe_prefix).second ||
        !app_ids.emplace(identity.installer_app_id).second ||
        !settings_app_ids.emplace(identity.settings_app_user_model_id).second) {
      std::cerr << "release identities are incomplete or collide\n";
      return 1;
    }
  }
  if (release_channel() != kReleaseIdentity.channel_name) {
    std::cerr << "selected release channel mismatch\n";
    return 1;
  }
  return 0;
}
