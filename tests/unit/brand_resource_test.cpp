#include <algorithm>
#include <array>
#include <cstdint>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <initializer_list>
#include <iostream>
#include <iterator>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

#if defined(_WIN32)
#include <windows.h>
#endif

namespace {

std::string read_text(const std::filesystem::path& path) {
    std::ifstream input(path, std::ios::binary);
    if (!input)
        throw std::runtime_error("could not open text file");
    return {std::istreambuf_iterator<char>(input), {}};
}

std::uint32_t little_endian(const unsigned char* bytes) {
    return static_cast<std::uint32_t>(bytes[0]) |
           (static_cast<std::uint32_t>(bytes[1]) << 8) |
           (static_cast<std::uint32_t>(bytes[2]) << 16) |
           (static_cast<std::uint32_t>(bytes[3]) << 24);
}

std::uint32_t big_endian(const unsigned char* bytes) {
    return (static_cast<std::uint32_t>(bytes[0]) << 24) |
           (static_cast<std::uint32_t>(bytes[1]) << 16) |
           (static_cast<std::uint32_t>(bytes[2]) << 8) |
           static_cast<std::uint32_t>(bytes[3]);
}

std::vector<int> ico_sizes(const std::filesystem::path& path) {
    std::ifstream input(path, std::ios::binary);
    if (!input)
        throw std::runtime_error("could not open ico file");
    std::uint16_t reserved = 1;
    std::uint16_t type = 0;
    std::uint16_t count = 0;
    input.read(reinterpret_cast<char*>(&reserved), sizeof(reserved));
    input.read(reinterpret_cast<char*>(&type), sizeof(type));
    input.read(reinterpret_cast<char*>(&count), sizeof(count));
    if (!input || reserved != 0 || type != 1 || count != 9)
        throw std::runtime_error("ICO must contain exactly nine frames");

    const auto file_size = std::filesystem::file_size(path);
    const std::uint64_t directory_end = 6u + 16u * count;
    if (file_size < directory_end)
        throw std::runtime_error("truncated ico directory");

    const std::array<unsigned char, 8> png_signature = {
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a,
    };
    const std::array<unsigned char, 4> ihdr_type = {'I', 'H', 'D', 'R'};
    std::array<std::array<unsigned char, 16>, 9> entries{};
    for (std::uint16_t index = 0; index < count; ++index) {
        input.read(reinterpret_cast<char*>(entries[index].data()), entries[index].size());
        if (!input)
            throw std::runtime_error("truncated ico directory");
    }

    std::vector<int> sizes;
    sizes.reserve(count);
    for (std::uint16_t index = 0; index < count; ++index) {
        const auto& entry = entries[index];
        const int width = entry[0] == 0 ? 256 : entry[0];
        const int height = entry[1] == 0 ? 256 : entry[1];
        if (width != height || (index != 0 && width <= sizes.back()))
            throw std::runtime_error("ICO frames are not ascending squares");

        const std::uint32_t bytes_in_frame = little_endian(&entry[8]);
        const std::uint32_t frame_offset = little_endian(&entry[12]);
        if (bytes_in_frame < 24 || frame_offset < directory_end ||
            static_cast<std::uint64_t>(frame_offset) + bytes_in_frame > file_size) {
            throw std::runtime_error("ICO frame is outside the file");
        }

        input.seekg(frame_offset);
        std::array<unsigned char, 24> png_header{};
        input.read(reinterpret_cast<char*>(png_header.data()), png_header.size());
        if (!input || !std::equal(png_signature.begin(), png_signature.end(), png_header.begin()) ||
            big_endian(&png_header[8]) != 13 ||
            !std::equal(ihdr_type.begin(), ihdr_type.end(), png_header.begin() + 12) ||
            big_endian(&png_header[16]) != static_cast<std::uint32_t>(width) ||
            big_endian(&png_header[20]) != static_cast<std::uint32_t>(height)) {
            throw std::runtime_error("ICO frame does not contain matching PNG pixels");
        }
        sizes.push_back(width);
    }
    return sizes;
}

std::vector<std::vector<unsigned char>> ico_frames(const std::filesystem::path& path) {
    std::ifstream input(path, std::ios::binary);
    if (!input)
        throw std::runtime_error("could not open ico file");
    const auto file_size = std::filesystem::file_size(path);
    std::vector<unsigned char> bytes(file_size);
    input.read(reinterpret_cast<char*>(bytes.data()), static_cast<std::streamsize>(bytes.size()));
    if (!input || bytes.size() < 6 || bytes[0] != 0 || bytes[1] != 0 || bytes[2] != 1 ||
        bytes[3] != 0) {
        throw std::runtime_error("ICO header is invalid");
    }
    const std::uint16_t count = static_cast<std::uint16_t>(bytes[4]) |
                                (static_cast<std::uint16_t>(bytes[5]) << 8);
    const std::size_t directory_end = 6u + 16u * count;
    if (count == 0 || bytes.size() < directory_end)
        throw std::runtime_error("ICO directory is invalid");
    std::vector<std::vector<unsigned char>> frames;
    frames.reserve(count);
    for (std::uint16_t index = 0; index < count; ++index) {
        const std::size_t entry = 6u + 16u * index;
        const std::uint32_t length = little_endian(bytes.data() + entry + 8);
        const std::uint32_t offset = little_endian(bytes.data() + entry + 12);
        if (offset < directory_end || static_cast<std::uint64_t>(offset) + length > bytes.size())
            throw std::runtime_error("ICO frame is outside the file");
        frames.emplace_back(bytes.begin() + offset, bytes.begin() + offset + length);
    }
    return frames;
}

bool contains_all(const std::string& text, std::initializer_list<const char*> required) {
    for (const char* value : required) {
        if (text.find(value) == std::string::npos)
            return false;
    }
    return true;
}

struct ArtifactPaths {
    std::optional<std::filesystem::path> launcher;
    std::optional<std::filesystem::path> tsf;
    std::vector<std::filesystem::path> normal;
};

ArtifactPaths parse_artifact_paths(int argc, char** argv) {
    ArtifactPaths paths;
    for (int index = 2; index < argc; ++index) {
        const std::string option = argv[index];
        if (index + 1 >= argc)
            throw std::runtime_error("brand artifact option needs a path");
        const std::filesystem::path value = argv[++index];
        if (option == "--launcher") {
            paths.launcher = value;
        } else if (option == "--tsf") {
            paths.tsf = value;
        } else if (option == "--normal") {
            paths.normal.push_back(value);
        } else {
            throw std::runtime_error("unknown brand artifact option: " + option);
        }
    }
    return paths;
}

#if defined(_WIN32)
void require_group_icon(const std::filesystem::path& path,
                        const char* label,
                        std::initializer_list<int> ids,
                        const std::vector<std::vector<unsigned char>>* expected_frames = nullptr) {
    if (!std::filesystem::is_regular_file(path))
        throw std::runtime_error(std::string(label) + " artifact is missing");
    const HMODULE module = LoadLibraryExW(
        path.c_str(), nullptr, LOAD_LIBRARY_AS_DATAFILE | LOAD_LIBRARY_AS_IMAGE_RESOURCE);
    if (!module)
        throw std::runtime_error(std::string("could not load ") + label + " as an image resource");
    for (const int id : ids) {
        if (!FindResourceW(module, MAKEINTRESOURCEW(id), RT_GROUP_ICON)) {
            FreeLibrary(module);
            throw std::runtime_error(std::string(label) + " is missing RT_GROUP_ICON resource " +
                                     std::to_string(id));
        }
    }
    if (expected_frames) {
        const HRSRC group = FindResourceW(module, MAKEINTRESOURCEW(*ids.begin()), RT_GROUP_ICON);
        const HGLOBAL loaded = LoadResource(module, group);
        const auto* bytes = static_cast<const unsigned char*>(LockResource(loaded));
        const DWORD size = SizeofResource(module, group);
        if (!bytes || size < 6 || bytes[0] != 0 || bytes[1] != 0 || bytes[2] != 1 ||
            bytes[3] != 0) {
            FreeLibrary(module);
            throw std::runtime_error(std::string(label) + " has an invalid RT_GROUP_ICON");
        }
        const std::uint16_t count = static_cast<std::uint16_t>(bytes[4]) |
                                    (static_cast<std::uint16_t>(bytes[5]) << 8);
        if (count != expected_frames->size() || size < 6u + 14u * count) {
            FreeLibrary(module);
            throw std::runtime_error(std::string(label) + " has an unexpected icon frame count");
        }
        for (std::uint16_t index = 0; index < count; ++index) {
            const unsigned char* entry = bytes + 6u + 14u * index;
            const WORD icon_id = static_cast<WORD>(entry[12]) |
                                 (static_cast<WORD>(entry[13]) << 8);
            const HRSRC icon = FindResourceW(module, MAKEINTRESOURCEW(icon_id), RT_ICON);
            const HGLOBAL icon_loaded = icon ? LoadResource(module, icon) : nullptr;
            const auto* icon_bytes = icon_loaded
                                         ? static_cast<const unsigned char*>(LockResource(icon_loaded))
                                         : nullptr;
            const DWORD icon_size = icon ? SizeofResource(module, icon) : 0;
            const auto& expected = (*expected_frames)[index];
            if (!icon_bytes || icon_size != expected.size() ||
                std::memcmp(icon_bytes, expected.data(), expected.size()) != 0) {
                FreeLibrary(module);
                throw std::runtime_error(std::string(label) +
                                         " does not contain the approved ICO frame data");
            }
        }
    }
    FreeLibrary(module);
}
#endif

} // namespace

int main(int argc, char** argv) {
    if (argc < 2) {
        std::cerr << "expected repository source root and optional artifact paths\n";
        return 1;
    }
    const std::filesystem::path root = argv[1];
    try {
        const ArtifactPaths artifacts = parse_artifact_paths(argc, argv);
        const std::vector<int> expected = {16, 20, 24, 32, 40, 48, 64, 128, 256};
        const auto product = ico_sizes(root / "resources/icons/fcitx5.ico");
        const auto product_frames = ico_frames(root / "resources/icons/fcitx5.ico");
        const auto paused = ico_sizes(root / "resources/icons/fcitx5-paused.ico");
        const auto error = ico_sizes(root / "resources/icons/fcitx5-error.ico");
        const auto tsf = ico_sizes(root / "resources/icons/fcitx5-tsf.ico");
        if (product != expected || paused != expected || error != expected || tsf != expected) {
            std::cerr << "approved penguin ICO sizes or order are incorrect\n";
            return 1;
        }
        const auto master = root / "resources/icons/source/fcitx5-master-approved.png";
        if (!std::filesystem::is_regular_file(master) || std::filesystem::file_size(master) == 0) {
            std::cerr << "approved penguin master is missing\n";
            return 1;
        }
        const auto preview = root / "resources/icons/fcitx5-icons-preview.png";
        if (!std::filesystem::exists(preview) || std::filesystem::file_size(preview) == 0) {
            std::cerr << "approved penguin preview is missing\n";
            return 1;
        }

        const auto appRc = read_text(root / "resources/windows/app.rc");
        const auto launcherRc = read_text(root / "resources/windows/launcher.rc");
        const auto tsfRc = read_text(root / "resources/windows/tsf.rc");
        const auto installer = read_text(root / "installer/fcitx5-windows.iss");
        const auto script = read_text(root / "resources/icons/generate_icons.py");
        const auto docs = read_text(root / "docs/brand-assets.md");
        if (!contains_all(appRc, {"IDI_FCITX5_APP ICON \"../icons/fcitx5.ico\""}) ||
            !contains_all(launcherRc, {
                "IDI_FCITX5_APP ICON \"../icons/fcitx5.ico\"",
                "IDI_FCITX5_PAUSED ICON \"../icons/fcitx5-paused.ico\"",
                "IDI_FCITX5_ERROR ICON \"../icons/fcitx5-error.ico\"",
            }) ||
            !contains_all(tsfRc, {"IDI_FCITX5_TSF ICON \"../icons/fcitx5-tsf.ico\""})) {
            std::cerr << "approved penguin ICO resources are not wired through RC files\n";
            return 1;
        }
        if (!contains_all(installer, {"SetupIconFile=..\\resources\\icons\\fcitx5.ico"})) {
            std::cerr << "installer is missing the approved penguin setup icon\n";
            return 1;
        }
        if (!contains_all(script, {
                "MASTER_SOURCE", "load_master", "square_frame", "render_master_icon",
                "add_status_badge", "write_ico", "ImageFilter.UnsharpMask",
                "ICO_SIZES = (16, 20, 24, 32, 40, 48, 64, 128, 256)",
                "BADGE_GLYPH = (255, 255, 255, 255)", "draw.line",
                "does not redraw or reinterpret the penguin",
            }) ||
            !contains_all(docs, {
                "user-approved scarf penguin artwork", "fcitx5-master-approved.png",
                "only visual authority", "original project branding artwork",
                "lower-right amber or red badge", "tighter face/scarf crop",
                "white pause bars", "white diagonal X",
                "deterministic alpha trimming", "does not redraw the penguin",
                "exactly these nine authored frames",
                "16, 20, 24, 32, 40, 48, 64, 128, 256",
                "not Explorer, taskbar, accessibility, or host evidence",
            })) {
            std::cerr << "approved penguin script or documentation constraints are incomplete\n";
            return 1;
        }
#if defined(_WIN32)
        if (artifacts.launcher) {
            require_group_icon(*artifacts.launcher, "launcher", {101, 102, 103});
        }
        for (const auto& normal : artifacts.normal) {
            require_group_icon(normal, "shipping executable", {101}, &product_frames);
        }
        if (artifacts.tsf) {
            require_group_icon(*artifacts.tsf, "TSF DLL", {104});
        }
#endif
    } catch (const std::exception& error) {
        std::cerr << error.what() << '\n';
        return 1;
    }
    return 0;
}
