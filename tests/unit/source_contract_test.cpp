#include <filesystem>
#include <fstream>
#include <iostream>
#include <iterator>
#include <string>

namespace {

std::string read_text(const std::filesystem::path& path) {
    std::ifstream input(path, std::ios::binary);
    if (!input)
        throw std::runtime_error("could not open source file");
    return {std::istreambuf_iterator<char>(input), {}};
}

int fail(const char* message) {
    std::cerr << message << '\n';
    return 1;
}

} // namespace

int main(int argc, char** argv) {
    if (argc != 2)
        return fail("expected repository source root");
    const std::filesystem::path sourceRoot = argv[1];
    const auto runtimeSource = read_text(sourceRoot / "src/engine/fcitx_runtime.cpp");
    const auto warmupMarker = runtimeSource.find("warmupIds");
    if (warmupMarker == std::string::npos)
        return fail("warmup section marker disappeared");
    const auto processKeyMarker = runtimeSource.find("RuntimeResult FcitxRuntime::processKey",
                                                     warmupMarker);
    if (processKeyMarker == std::string::npos)
        return fail("processKey marker disappeared");
    const auto warmupSection =
        runtimeSource.substr(warmupMarker, processKeyMarker - warmupMarker);
    if (warmupSection.find("keyEvent(") != std::string::npos ||
        warmupSection.find("FcitxKey_") != std::string::npos) {
        return fail("REG-WARMUP-001: generic warmup must not synthesize text key events");
    }
    const auto uiSource = read_text(sourceRoot / "src/ui/ui_main.cpp");
    if (uiSource.find("L\"zh-CN\", &format") != std::string::npos) {
        return fail("REG-PROFILE-001: candidate DWrite locale must not be hardcoded to zh-CN");
    }
    std::cout << "source-contract ok\n";
    return 0;
}
