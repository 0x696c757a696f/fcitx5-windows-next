#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>

#include <miniz.h>

typedef struct Fcitx5MinizArchive {
  FILE* file;
  mz_zip_archive archive;
  int initialized;
} Fcitx5MinizArchive;

typedef struct Fcitx5MinizEntry {
  char name[MZ_ZIP_MAX_ARCHIVE_FILENAME_SIZE];
  uint64_t uncompressed_size;
  uint32_t directory;
  uint32_t encrypted;
  uint32_t supported;
  uint32_t unix_symlink;
} Fcitx5MinizEntry;

static int fcitx5_is_unix_symlink(const mz_zip_archive_file_stat* stat) {
  const mz_uint32 unix_host = 3U;
  const mz_uint32 file_type_mask = 0170000U;
  const mz_uint32 symbolic_link = 0120000U;
  const mz_uint32 host = (mz_uint32)(stat->m_version_made_by >> 8U);
  const mz_uint32 mode = stat->m_external_attr >> 16U;
  return host == unix_host && (mode & file_type_mask) == symbolic_link;
}

int fcitx5_miniz_open_utf16(const wchar_t* path,
                            uint64_t maximum_archive_bytes,
                            Fcitx5MinizArchive** out_archive) {
  if (path == NULL || out_archive == NULL) {
    return 0;
  }
  *out_archive = NULL;
  FILE* file = NULL;
  if (_wfopen_s(&file, path, L"rb") != 0 || file == NULL) {
    return 0;
  }
  if (_fseeki64(file, 0, SEEK_END) != 0) {
    fclose(file);
    return 0;
  }
  const __int64 measured = _ftelli64(file);
  if (measured <= 0 || (uint64_t)measured > maximum_archive_bytes ||
      _fseeki64(file, 0, SEEK_SET) != 0) {
    fclose(file);
    return 0;
  }
  Fcitx5MinizArchive* archive =
      (Fcitx5MinizArchive*)calloc(1, sizeof(Fcitx5MinizArchive));
  if (archive == NULL) {
    fclose(file);
    return 0;
  }
  archive->file = file;
  if (mz_zip_reader_init_cfile(&archive->archive, file, (mz_uint64)measured, 0) != MZ_TRUE) {
    fclose(file);
    free(archive);
    return 0;
  }
  archive->initialized = 1;
  *out_archive = archive;
  return 1;
}

void fcitx5_miniz_close(Fcitx5MinizArchive* archive) {
  if (archive == NULL) {
    return;
  }
  if (archive->initialized) {
    mz_zip_reader_end(&archive->archive);
  }
  if (archive->file != NULL) {
    fclose(archive->file);
  }
  free(archive);
}

uint32_t fcitx5_miniz_num_files(Fcitx5MinizArchive* archive) {
  if (archive == NULL) {
    return 0;
  }
  return (uint32_t)mz_zip_reader_get_num_files(&archive->archive);
}

int fcitx5_miniz_stat(Fcitx5MinizArchive* archive,
                      uint32_t index,
                      Fcitx5MinizEntry* out_entry) {
  if (archive == NULL || out_entry == NULL) {
    return 0;
  }
  mz_zip_archive_file_stat stat;
  memset(&stat, 0, sizeof(stat));
  if (mz_zip_reader_file_stat(&archive->archive, (mz_uint)index, &stat) != MZ_TRUE) {
    return 0;
  }
  memset(out_entry, 0, sizeof(*out_entry));
  strncpy_s(out_entry->name, sizeof(out_entry->name), stat.m_filename, _TRUNCATE);
  out_entry->uncompressed_size = (uint64_t)stat.m_uncomp_size;
  out_entry->directory = stat.m_is_directory == MZ_TRUE ? 1U : 0U;
  out_entry->encrypted = stat.m_is_encrypted == MZ_TRUE ? 1U : 0U;
  out_entry->supported = stat.m_is_supported == MZ_TRUE ? 1U : 0U;
  out_entry->unix_symlink = fcitx5_is_unix_symlink(&stat) ? 1U : 0U;
  return 1;
}

int fcitx5_miniz_locate(Fcitx5MinizArchive* archive,
                        const char* name,
                        uint32_t* out_index) {
  if (archive == NULL || name == NULL || out_index == NULL) {
    return 0;
  }
  const int found = mz_zip_reader_locate_file(&archive->archive, name, NULL,
                                              MZ_ZIP_FLAG_CASE_SENSITIVE);
  if (found < 0) {
    return 0;
  }
  *out_index = (uint32_t)found;
  return 1;
}

int fcitx5_miniz_validate(Fcitx5MinizArchive* archive, uint32_t index) {
  if (archive == NULL) {
    return 0;
  }
  return mz_zip_validate_file(&archive->archive, (mz_uint)index, 0) == MZ_TRUE ? 1 : 0;
}

int fcitx5_miniz_extract(Fcitx5MinizArchive* archive,
                         uint32_t index,
                         uint8_t* output,
                         size_t output_size) {
  if (archive == NULL || (output == NULL && output_size != 0U)) {
    return 0;
  }
  return mz_zip_reader_extract_to_mem(&archive->archive, (mz_uint)index, output,
                                      output_size, 0) == MZ_TRUE
             ? 1
             : 0;
}
