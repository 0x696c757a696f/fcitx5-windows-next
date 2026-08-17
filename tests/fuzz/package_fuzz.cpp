#include "package_core.h"

#include <windows.h>

#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <random>
#include <span>
#include <string>
#include <string_view>
#include <vector>

#include <miniz.h>

namespace {

void consume(std::span<const std::uint8_t> bytes) {
  const std::string_view text(reinterpret_cast<const char*>(bytes.data()), bytes.size());
  static_cast<void>(fcitx::package::is_safe_relative_package_path(text));
  try {
    static_cast<void>(fcitx::package::parse_manifest(text));
  } catch (const fcitx::package::PackageError&) {
  }

  mz_zip_archive archive{};
  if (!bytes.empty() && mz_zip_reader_init_mem(&archive, bytes.data(), bytes.size(), 0) == MZ_TRUE) {
    const auto count = mz_zip_reader_get_num_files(&archive);
    if (count <= fcitx::package::kMaximumFileCount + 2U) {
      for (mz_uint index = 0; index < count; ++index) {
        mz_zip_archive_file_stat stat{};
        if (mz_zip_reader_file_stat(&archive, index, &stat) != MZ_TRUE) break;
        const std::string_view name(
            stat.m_filename,
            strnlen_s(stat.m_filename, MZ_ZIP_MAX_ARCHIVE_FILENAME_SIZE));
        if (name.starts_with("payload/")) {
          static_cast<void>(fcitx::package::is_safe_relative_package_path(name.substr(8U)));
        }
        if (stat.m_uncomp_size > fcitx::package::kMaximumFileBytes) break;
        static_cast<void>(mz_zip_validate_file(&archive, index, 0));
      }
    }
    static_cast<void>(mz_zip_reader_end(&archive));
  }
}

}  // namespace

extern "C" int LLVMFuzzerTestOneInput(const std::uint8_t* data, std::size_t size) {
  if (size <= fcitx::package::kMaximumManifestBytes) consume(std::span(data, size));
  return 0;
}

int main(int argc, char** argv) {
  if (argc > 1) {
    for (int index = 1; index < argc; ++index) {
      std::ifstream input(argv[index], std::ios::binary);
      std::vector<std::uint8_t> bytes((std::istreambuf_iterator<char>(input)),
                                      std::istreambuf_iterator<char>());
      LLVMFuzzerTestOneInput(bytes.data(), bytes.size());
    }
    return 0;
  }
  std::mt19937_64 random(0x5041434B414745ULL);
  std::vector<std::uint8_t> bytes;
  for (std::size_t iteration = 0; iteration < 20'000U; ++iteration) {
    bytes.resize(static_cast<std::size_t>(random() % 4096U));
    for (auto& byte : bytes) byte = static_cast<std::uint8_t>(random());
    LLVMFuzzerTestOneInput(bytes.data(), bytes.size());
  }
  return 0;
}
