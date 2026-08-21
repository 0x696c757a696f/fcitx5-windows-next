#include "package_core.h"

#include <fcitx5_windows/release_identity.h>
#include <fcitx5_windows/version.h>

#include <filesystem>
#include <fstream>
#include <iostream>
#include <span>
#include <string>
#include <string_view>
#include <vector>

namespace {

std::string read_file(const std::filesystem::path& path, std::size_t maximum) {
  std::error_code error;
  const auto size = std::filesystem::file_size(path, error);
  if (error || size > maximum) {
    throw fcitx::package::PackageError("invalid_file", "input is missing or too large");
  }
  std::ifstream input(path, std::ios::binary);
  std::string bytes(static_cast<std::size_t>(size), '\0');
  if (!bytes.empty()) input.read(bytes.data(), static_cast<std::streamsize>(bytes.size()));
  if (!input) throw fcitx::package::PackageError("io_error", "input read failed");
  return bytes;
}

void print_package(const fcitx::package::Manifest& manifest, bool verified) {
  std::cout << "id=" << manifest.id << '\n'
            << "version=" << manifest.version << '\n'
            << "source_commit=" << manifest.source_commit << '\n'
            << "license=" << manifest.license << '\n'
            << "key_id=" << manifest.key_id << '\n'
            << "signature_verified=" << (verified ? "true" : "false") << '\n'
            << "permissions=";
  for (std::size_t index = 0; index < manifest.permissions.size(); ++index) {
    if (index != 0U) std::cout << ',';
    std::cout << manifest.permissions[index];
  }
  std::cout << '\n';
}

int usage() {
  std::wcerr << L"Usage:\n"
                L"  fcitx5-package --version\n"
                L"  fcitx5-package --validate-manifest MANIFEST\n"
                L"  fcitx5-package --validate-keyring KEYRING\n"
                L"  fcitx5-package --install ARCHIVE INSTALL_ROOT TRANSACTION_ID KEYRING\n"
                L"  fcitx5-package --repair INSTALL_ROOT KEYRING\n"
                L"  fcitx5-package --state INSTALL_ROOT PACKAGE_ID STATE\n"
                L"  fcitx5-package --list INSTALL_ROOT\n"
                L"  fcitx5-package --verify-repository INDEX SIGNATURE KEYRING\n"
                L"  fcitx5-package --verify-repository-v2 INDEX SIG_JSON KEYRING [CHANNEL]\n"
                L"  fcitx5-package --verify-manifest-v2 MANIFEST SIG_JSON KEYRING\n"
                L"  fcitx5-package --mark-remove INSTALL_ROOT PACKAGE_ID\n"
                L"  fcitx5-package --finalize-remove INSTALL_ROOT PACKAGE_ID\n";
  return 1;
}

}  // namespace

int wmain(int argc, wchar_t** argv) {
  try {
    using namespace fcitx::package;
    if (argc == 2 && std::wstring_view(argv[1]) == L"--version") {
      std::cout << "fcitx5-package " << fcitx::windows::version() << '\n';
      return 0;
    }
    if (argc == 3 && std::wstring_view(argv[1]) == L"--validate-manifest") {
      print_package(parse_manifest(read_file(argv[2], kMaximumManifestBytes)), false);
      return 0;
    }
    if (argc == 3 && std::wstring_view(argv[1]) == L"--validate-keyring") {
      const auto keys = read_trusted_keys(argv[2]);
      if (keys.empty()) throw PackageError("invalid_keyring", "trusted keyring is empty");
      std::cout << "keys=" << keys.size() << '\n';
      return 0;
    }
    if (argc == 6 && std::wstring_view(argv[1]) == L"--install") {
      const auto keys = read_trusted_keys(argv[5]);
      const auto staged = stage_verified_archive(argv[2], argv[3],
                                                 std::filesystem::path(argv[4]).string(), keys);
      const auto manifest_bytes = read_file(staged / "manifest.json", kMaximumManifestBytes);
      const auto manifest = parse_manifest(manifest_bytes);
      activate_staged_package(staged, argv[3], keys);
      print_package(manifest, true);
      return 0;
    }
    if (argc == 4 && std::wstring_view(argv[1]) == L"--repair") {
      const auto keys = read_trusted_keys(argv[3]);
      verify_installed_packages(argv[2], keys);
      std::cout << "repair=verified\n";
      return 0;
    }
    if (argc == 5 && std::wstring_view(argv[1]) == L"--state") {
      set_package_state(argv[2], std::filesystem::path(argv[3]).string(),
                        std::filesystem::path(argv[4]).string());
      return 0;
    }
    if (argc == 3 && std::wstring_view(argv[1]) == L"--list") {
      for (const auto& entry : read_lockfile(argv[2])) {
        std::cout << entry.id << '\t' << entry.version << '\t' << entry.state << '\t'
                  << entry.manifest_sha256 << '\n';
      }
      return 0;
    }
    if ((argc == 5 || argc == 6) && std::wstring_view(argv[1]) == L"--verify-repository") {
      const auto index_bytes = read_file(argv[2], kMaximumManifestBytes);
      const auto signature_bytes = read_file(argv[3], 16U * 1024U);
      // Optional expected channel (defaults to this build's release channel);
      // the index must match it exactly.
      const std::string expectedChannel =
          argc == 6 ? std::filesystem::path(argv[5]).string()
                    : std::string(fcitx::windows::kReleaseIdentity.channel_name);
      const auto index = verify_repository_index(
          index_bytes, std::as_bytes(std::span(signature_bytes)),
          read_trusted_keys(argv[4]), expectedChannel);
      for (const auto& entry : index.packages) {
        std::cout << entry.id << '\t' << entry.version << '\t' << entry.release_sequence << '\t'
                  << entry.architecture << '\t' << entry.sha256 << '\t' << entry.download_url
                  << '\t' << entry.title << '\n';
      }
      return 0;
    }
    if ((argc == 5 || argc == 6) && std::wstring_view(argv[1]) == L"--verify-repository-v2") {
      const auto index_bytes = read_file(argv[2], kMaximumManifestBytes);
      const auto envelope =
          read_signature_envelope(argv[3], "repository-index");
      const std::string expectedChannel =
          argc == 6 ? std::filesystem::path(argv[5]).string()
                    : std::string(fcitx::windows::kReleaseIdentity.channel_name);
      const auto index = verify_repository_index(index_bytes, envelope, read_trusted_keys(argv[4]),
                                                 expectedChannel);
      for (const auto& entry : index.packages) {
        std::cout << entry.id << '\t' << entry.version << '\t' << entry.release_sequence << '\t'
                  << entry.architecture << '\t' << entry.sha256 << '\t' << entry.download_url
                  << '\t' << entry.title << '\n';
      }
      return 0;
    }
    if (argc == 5 && std::wstring_view(argv[1]) == L"--verify-manifest-v2") {
      verify_manifest_signature_envelope(
          read_file(argv[2], kMaximumManifestBytes),
          read_signature_envelope(argv[3], "package-manifest"),
          read_trusted_keys(argv[4]));
      std::cout << "manifest_signature=verified\n";
      return 0;
    }
    if (argc == 4 && std::wstring_view(argv[1]) == L"--mark-remove") {
      mark_package_for_removal(argv[2], std::filesystem::path(argv[3]).string());
      return 0;
    }
    if (argc == 4 && std::wstring_view(argv[1]) == L"--finalize-remove") {
      finalize_package_removal(argv[2], std::filesystem::path(argv[3]).string());
      return 0;
    }
    return usage();
  } catch (const fcitx::package::PackageError& error) {
    std::cerr << error.code() << ": " << error.what() << '\n';
    return 2;
  } catch (const std::exception& error) {
    std::cerr << "internal_error: " << error.what() << '\n';
    return 3;
  }
}
