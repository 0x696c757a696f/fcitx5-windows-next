#include "package_core.h"

#include <windows.h>

#include <cstdio>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <limits>
#include <map>
#include <set>
#include <string>
#include <string_view>
#include <system_error>
#include <vector>

#include <miniz.h>

namespace fcitx::package {
namespace {

constexpr std::uint64_t kMaximumArchiveBytes = 128ULL * 1024ULL * 1024ULL;
constexpr std::size_t kMaximumSignatureBytes = 16U * 1024U;

[[noreturn]] void archive_fail(std::string code, std::string message) {
  throw PackageError(std::move(code), std::move(message));
}

class ArchiveReader final {
 public:
  explicit ArchiveReader(const std::filesystem::path& path) {
    if (_wfopen_s(&file_, path.c_str(), L"rb") != 0 || file_ == nullptr) {
      archive_fail("invalid_archive", "unable to open package archive");
    }
    if (_fseeki64(file_, 0, SEEK_END) != 0) {
      archive_fail("invalid_archive", "unable to measure package archive");
    }
    const auto measured = _ftelli64(file_);
    if (measured <= 0 || static_cast<std::uint64_t>(measured) > kMaximumArchiveBytes ||
        _fseeki64(file_, 0, SEEK_SET) != 0) {
      archive_fail("invalid_archive", "package archive exceeds its resource budget");
    }
    if (mz_zip_reader_init_cfile(&archive_, file_, static_cast<mz_uint64>(measured), 0) !=
        MZ_TRUE) {
      archive_fail("invalid_archive", "ZIP central directory is invalid");
    }
    initialized_ = true;
  }

  ~ArchiveReader() {
    if (initialized_) {
      static_cast<void>(mz_zip_reader_end(&archive_));
    }
    if (file_ != nullptr) {
      fclose(file_);
    }
  }
  ArchiveReader(const ArchiveReader&) = delete;
  ArchiveReader& operator=(const ArchiveReader&) = delete;

  [[nodiscard]] mz_zip_archive* get() noexcept { return &archive_; }

 private:
  FILE* file_{};
  mz_zip_archive archive_{};
  bool initialized_{};
};

std::wstring utf8_to_wide(std::string_view value) {
  if (value.empty() || value.size() > static_cast<std::size_t>(std::numeric_limits<int>::max())) {
    archive_fail("unsafe_archive_path", "archive path is empty or too long");
  }
  const auto input_size = static_cast<int>(value.size());
  const int required = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, value.data(), input_size,
                                           nullptr, 0);
  if (required <= 0) {
    archive_fail("unsafe_archive_path", "archive path is not valid UTF-8");
  }
  std::wstring result(static_cast<std::size_t>(required), L'\0');
  if (MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, value.data(), input_size, result.data(),
                          required) != required) {
    archive_fail("unsafe_archive_path", "archive path conversion failed");
  }
  return result;
}

struct OrdinalIgnoreCaseLess {
  bool operator()(const std::wstring& left, const std::wstring& right) const noexcept {
    return CompareStringOrdinal(left.data(), static_cast<int>(left.size()), right.data(),
                                static_cast<int>(right.size()), TRUE) == CSTR_LESS_THAN;
  }
};

std::string checked_filename(const mz_zip_archive_file_stat& stat) {
  const auto length = strnlen_s(stat.m_filename, MZ_ZIP_MAX_ARCHIVE_FILENAME_SIZE);
  if (length == 0U || length >= MZ_ZIP_MAX_ARCHIVE_FILENAME_SIZE - 1U) {
    archive_fail("unsafe_archive_path", "archive filename is empty or truncated");
  }
  return std::string(stat.m_filename, length);
}

bool is_unix_symlink(const mz_zip_archive_file_stat& stat) noexcept {
  constexpr mz_uint32 kUnixHost = 3U;
  constexpr mz_uint32 kFileTypeMask = 0170000U;
  constexpr mz_uint32 kSymbolicLink = 0120000U;
  const auto host = static_cast<mz_uint32>(stat.m_version_made_by >> 8U);
  const auto mode = stat.m_external_attr >> 16U;
  return host == kUnixHost && (mode & kFileTypeMask) == kSymbolicLink;
}

mz_uint find_exact_entry(mz_zip_archive* archive, std::string_view name) {
  const int found = mz_zip_reader_locate_file(archive, std::string(name).c_str(), nullptr,
                                              MZ_ZIP_FLAG_CASE_SENSITIVE);
  if (found < 0) {
    archive_fail("invalid_archive", "required archive entry is missing: " + std::string(name));
  }
  return static_cast<mz_uint>(found);
}

std::vector<std::byte> extract_entry(mz_zip_archive* archive, mz_uint index,
                                     std::size_t maximum_size) {
  mz_zip_archive_file_stat stat{};
  if (mz_zip_reader_file_stat(archive, index, &stat) != MZ_TRUE || stat.m_is_directory ||
      stat.m_is_encrypted || stat.m_is_supported != MZ_TRUE ||
      stat.m_uncomp_size > maximum_size ||
      stat.m_uncomp_size > static_cast<mz_uint64>(std::numeric_limits<std::size_t>::max())) {
    archive_fail("invalid_archive", "archive entry violates type or size constraints");
  }
  std::vector<std::byte> result(static_cast<std::size_t>(stat.m_uncomp_size));
  if (mz_zip_reader_extract_to_mem(archive, index, result.data(), result.size(), 0) != MZ_TRUE) {
    archive_fail("invalid_archive", "archive entry failed integrity validation");
  }
  return result;
}

void write_binary_file(const std::filesystem::path& path, std::span<const std::byte> bytes) {
  std::ofstream output(path, std::ios::binary | std::ios::trunc);
  if (!output) {
    archive_fail("io_error", "unable to create extracted file");
  }
  if (!bytes.empty()) {
    output.write(reinterpret_cast<const char*>(bytes.data()),
                 static_cast<std::streamsize>(bytes.size()));
  }
  if (!output) {
    archive_fail("io_error", "unable to write extracted file");
  }
}

}  // namespace

std::filesystem::path stage_verified_archive(
    const std::filesystem::path& archive_path, const std::filesystem::path& install_root,
    std::string_view transaction_id, std::span<const TrustedKey> trusted_keys) {
  if (!is_safe_relative_package_path(transaction_id) || transaction_id.find('/') !=
                                                         std::string_view::npos ||
      path_contains_reparse_point(archive_path) ||
      path_contains_reparse_point(install_root)) {
    archive_fail("unsafe_path", "transaction id is invalid");
  }
  ArchiveReader reader(archive_path);
  auto* archive = reader.get();
  const auto entry_count = mz_zip_reader_get_num_files(archive);
  if (entry_count < 3U || entry_count > kMaximumFileCount + 2U) {
    archive_fail("invalid_archive", "archive entry count is outside its budget");
  }

  const auto manifest_index = find_exact_entry(archive, "manifest.json");
  const auto signature_index = find_exact_entry(archive, "manifest.sig");
  const auto manifest_blob = extract_entry(archive, manifest_index, kMaximumManifestBytes);
  const auto signature = extract_entry(archive, signature_index, kMaximumSignatureBytes);
  const std::string manifest_bytes(reinterpret_cast<const char*>(manifest_blob.data()),
                                   manifest_blob.size());
  const auto manifest = parse_manifest(manifest_bytes);
#if defined(_WIN64)
  validate_manifest_compatibility(manifest, "x64");
#else
  validate_manifest_compatibility(manifest, "x86");
#endif
  const auto key = std::ranges::find_if(trusted_keys, [&](const TrustedKey& candidate) {
    return candidate.id == manifest.key_id;
  });
  if (key == trusted_keys.end()) {
    archive_fail("untrusted_key", "manifest key is not in the trusted key set");
  }
  verify_manifest_signature(manifest_bytes, signature, *key);

  std::map<std::string, const FileEntry*, std::less<>> expected_files;
  std::set<std::wstring, OrdinalIgnoreCaseLess> expected_casefold;
  expected_casefold.emplace(L"manifest.json");
  expected_casefold.emplace(L"manifest.sig");
  for (const auto& file : manifest.files) {
    const auto archive_name = "payload/" + file.path;
    expected_files.emplace(archive_name, &file);
    if (!expected_casefold.emplace(utf8_to_wide(archive_name)).second) {
      archive_fail("unsafe_archive_path", "manifest paths collide on Windows");
    }
  }

  std::set<std::wstring, OrdinalIgnoreCaseLess> seen;
  std::uint64_t total_uncompressed = 0;
  for (mz_uint index = 0; index < entry_count; ++index) {
    mz_zip_archive_file_stat stat{};
    if (mz_zip_reader_file_stat(archive, index, &stat) != MZ_TRUE || stat.m_is_encrypted ||
        stat.m_is_supported != MZ_TRUE || is_unix_symlink(stat)) {
      archive_fail("invalid_archive", "archive contains an unsupported or executable link entry");
    }
    const auto name = checked_filename(stat);
    auto logical_name = name;
    if (stat.m_is_directory == MZ_TRUE && logical_name.back() == '/') {
      logical_name.pop_back();
    }
    if (logical_name != "manifest.json" && logical_name != "manifest.sig") {
      if (!logical_name.starts_with("payload/") ||
          !is_safe_relative_package_path(std::string_view(logical_name).substr(8U))) {
        archive_fail("unsafe_archive_path", "archive path is outside payload/");
      }
    }
    if (!seen.emplace(utf8_to_wide(logical_name)).second) {
      archive_fail("unsafe_archive_path", "archive contains a case-insensitive duplicate path");
    }
    if (stat.m_is_directory == MZ_TRUE) {
      continue;
    }
    if (!expected_casefold.contains(utf8_to_wide(logical_name))) {
      archive_fail("invalid_archive", "archive contains an undeclared file");
    }
    const auto expected = expected_files.find(logical_name);
    if (expected != expected_files.end() && expected->second->size != stat.m_uncomp_size) {
      archive_fail("payload_mismatch", "archive file size differs from manifest");
    }
    if (stat.m_uncomp_size > kMaximumFileBytes ||
        total_uncompressed > kMaximumPayloadBytes - stat.m_uncomp_size) {
      archive_fail("invalid_archive", "archive expands beyond its resource budget");
    }
    total_uncompressed += stat.m_uncomp_size;
    if (mz_zip_validate_file(archive, index, 0) != MZ_TRUE) {
      archive_fail("invalid_archive", "archive entry integrity validation failed");
    }
  }
  if (seen.size() < expected_casefold.size()) {
    archive_fail("invalid_archive", "archive is missing a declared payload file");
  }

  std::filesystem::create_directories(install_root / "staging");
  const auto extraction = install_root / "staging" /
                          std::filesystem::path(std::string(transaction_id) + ".extract");
  const auto staged = install_root / "staging" / std::filesystem::path(transaction_id);
  if (std::filesystem::exists(extraction) || std::filesystem::exists(staged)) {
    archive_fail("transaction_exists", "staging transaction already exists");
  }
  try {
    std::filesystem::create_directories(extraction / "payload");
    write_binary_file(extraction / "manifest.json", manifest_blob);
    write_binary_file(extraction / "manifest.sig", signature);
    for (const auto& [archive_name, file] : expected_files) {
      const auto index = find_exact_entry(archive, archive_name);
      const auto contents = extract_entry(archive, index, static_cast<std::size_t>(file->size));
      const auto output = extraction / "payload" / std::filesystem::path(file->path);
      std::filesystem::create_directories(output.parent_path());
      write_binary_file(output, contents);
    }
    verify_payload(manifest, extraction / "payload");
    if (!MoveFileExW(extraction.c_str(), staged.c_str(), MOVEFILE_WRITE_THROUGH)) {
      archive_fail("activation_failed", "unable to publish verified staging directory");
    }
  } catch (...) {
    std::error_code ignored;
    std::filesystem::remove_all(extraction, ignored);
    throw;
  }
  return staged;
}

}  // namespace fcitx::package
