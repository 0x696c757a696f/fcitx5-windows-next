#include <cstdint>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <iterator>
#include <set>
#include <string>

namespace {

std::string read_text(const std::filesystem::path& path) {
    std::ifstream input(path, std::ios::binary);
    if (!input)
        throw std::runtime_error("could not open text file");
    return {std::istreambuf_iterator<char>(input), {}};
}

std::set<int> ico_sizes(const std::filesystem::path& path) {
    std::ifstream input(path, std::ios::binary);
    if (!input)
        throw std::runtime_error("could not open ico file");
    std::uint16_t reserved = 1;
    std::uint16_t type = 0;
    std::uint16_t count = 0;
    input.read(reinterpret_cast<char*>(&reserved), sizeof(reserved));
    input.read(reinterpret_cast<char*>(&type), sizeof(type));
    input.read(reinterpret_cast<char*>(&count), sizeof(count));
    if (!input || reserved != 0 || type != 1 || count == 0 || count > 64)
        throw std::runtime_error("invalid ico header");
    std::set<int> sizes;
    for (std::uint16_t index = 0; index < count; ++index) {
        unsigned char entry[16]{};
        input.read(reinterpret_cast<char*>(entry), sizeof(entry));
        if (!input)
            throw std::runtime_error("truncated ico directory");
        const int width = entry[0] == 0 ? 256 : entry[0];
        const int height = entry[1] == 0 ? 256 : entry[1];
        if (width != height)
            throw std::runtime_error("non-square ico image");
        sizes.insert(width);
    }
    return sizes;
}

bool contains_all(const std::set<int>& values, std::initializer_list<int> required) {
    for (const int value : required) {
        if (!values.contains(value))
            return false;
    }
    return true;
}

} // namespace

int main(int argc, char** argv) {
    if (argc != 2) {
        std::cerr << "expected repository source root\n";
        return 1;
    }
    const std::filesystem::path root = argv[1];
    try {
        const auto product = ico_sizes(root / "resources/icons/fcitx5.ico");
        const auto tsf = ico_sizes(root / "resources/icons/fcitx5-tsf.ico");
        if (!contains_all(product, {16, 20, 24, 32, 48, 256}) ||
            !contains_all(tsf, {16, 20, 24, 32, 48, 256})) {
            std::cerr << "product or TSF ico is missing required shell sizes\n";
            return 1;
        }
        const auto appRc = read_text(root / "resources/windows/app.rc");
        const auto tsfRc = read_text(root / "resources/windows/tsf.rc");
        const auto script = read_text(root / "resources/icons/generate_icons.py");
        const auto docs = read_text(root / "docs/brand-assets.md");
        if (appRc.find("fcitx5.ico") == std::string::npos ||
            tsfRc.find("fcitx5-tsf.ico") == std::string::npos ||
            script.find("micro_penguin") == std::string::npos ||
            script.find("third-party") == std::string::npos ||
            docs.find("original geometric artwork") == std::string::npos ||
            docs.find("no language characters") == std::string::npos) {
            std::cerr << "brand resources are not wired through the documented penguin asset pipeline\n";
            return 1;
        }
    } catch (const std::exception& error) {
        std::cerr << error.what() << '\n';
        return 1;
    }
    return 0;
}
