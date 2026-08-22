#include <algorithm>
#include <cctype>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <iterator>
#include <string>
#include <vector>

namespace {

uint16_t readU16(const std::vector<uint8_t>& image, size_t offset) {
    if (offset + 2 > image.size()) throw std::runtime_error("truncated u16");
    return static_cast<uint16_t>(image[offset]) |
           (static_cast<uint16_t>(image[offset + 1]) << 8);
}

uint32_t readU32(const std::vector<uint8_t>& image, size_t offset) {
    if (offset + 4 > image.size()) throw std::runtime_error("truncated u32");
    return static_cast<uint32_t>(image[offset]) |
           (static_cast<uint32_t>(image[offset + 1]) << 8) |
           (static_cast<uint32_t>(image[offset + 2]) << 16) |
           (static_cast<uint32_t>(image[offset + 3]) << 24);
}

std::string readCString(const std::vector<uint8_t>& image, size_t offset) {
    std::string value;
    while (offset < image.size() && image[offset] != 0) {
        value.push_back(static_cast<char>(image[offset++]));
    }
    if (offset >= image.size()) throw std::runtime_error("unterminated string");
    std::ranges::transform(value, value.begin(), [](unsigned char c) {
        return static_cast<char>(std::tolower(c));
    });
    return value;
}

struct Section {
    uint32_t virtualAddress;
    uint32_t virtualSize;
    uint32_t rawPointer;
    uint32_t rawSize;
};

size_t rvaToOffset(uint32_t rva, const std::vector<Section>& sections) {
    for (const auto& section : sections) {
        const uint32_t size = std::max(section.virtualSize, section.rawSize);
        if (rva >= section.virtualAddress && rva < section.virtualAddress + size) {
            return section.rawPointer + (rva - section.virtualAddress);
        }
    }
    throw std::runtime_error("RVA outside sections");
}

struct PeAudit {
    uint16_t machine = 0;
    uint16_t optionalMagic = 0;
    uint16_t majorOperatingSystem = 0;
    uint16_t majorSubsystem = 0;
    uint16_t dllCharacteristics = 0;
    std::vector<std::string> imports;
};

PeAudit parsePe(const std::vector<uint8_t>& image) {
    if (image.size() < 0x100 || readU16(image, 0) != 0x5a4d) {
        throw std::runtime_error("not an MZ image");
    }
    const uint32_t peOffset = readU32(image, 0x3c);
    if (peOffset + 24 > image.size() || readU32(image, peOffset) != 0x00004550) {
        throw std::runtime_error("not a PE image");
    }
    PeAudit audit;
    audit.machine = readU16(image, peOffset + 4);
    const uint16_t sectionCount = readU16(image, peOffset + 6);
    const uint16_t optionalHeaderSize = readU16(image, peOffset + 20);
    const size_t optionalOffset = peOffset + 24;
    const size_t sectionOffset = optionalOffset + optionalHeaderSize;
    if (sectionOffset + static_cast<size_t>(sectionCount) * 40 > image.size()) {
        throw std::runtime_error("truncated section table");
    }
    audit.optionalMagic = readU16(image, optionalOffset);
    if (audit.optionalMagic != 0x10b && audit.optionalMagic != 0x20b) {
        throw std::runtime_error("unsupported optional header");
    }
    audit.majorOperatingSystem = readU16(image, optionalOffset + 40);
    audit.majorSubsystem = readU16(image, optionalOffset + 48);
    audit.dllCharacteristics = readU16(image, optionalOffset + 70);
    const size_t dataDirectoryOffset = optionalOffset + (audit.optionalMagic == 0x20b ? 112 : 96);
    const uint32_t importRva = readU32(image, dataDirectoryOffset + 8);
    std::vector<Section> sections;
    for (uint16_t i = 0; i < sectionCount; ++i) {
        const size_t offset = sectionOffset + static_cast<size_t>(i) * 40;
        sections.push_back({
            readU32(image, offset + 12),
            readU32(image, offset + 8),
            readU32(image, offset + 20),
            readU32(image, offset + 16),
        });
    }
    if (importRva == 0) return audit;
    size_t descriptorOffset = rvaToOffset(importRva, sections);
    for (;;) {
        const uint32_t originalFirstThunk = readU32(image, descriptorOffset);
        const uint32_t nameRva = readU32(image, descriptorOffset + 12);
        const uint32_t firstThunk = readU32(image, descriptorOffset + 16);
        if (originalFirstThunk == 0 && nameRva == 0 && firstThunk == 0) break;
        audit.imports.push_back(readCString(image, rvaToOffset(nameRva, sections)));
        descriptorOffset += 20;
    }
    return audit;
}

bool hasImport(const PeAudit& audit, const char* needle) {
    return std::ranges::find(audit.imports, needle) != audit.imports.end();
}

bool hasForbiddenProductImport(const PeAudit& audit) {
    for (const auto& import : audit.imports) {
        if (import.find("fcitx") != std::string::npos ||
            import.find("libime") != std::string::npos ||
            import.find("candidate") != std::string::npos ||
            import.find("package") != std::string::npos ||
            import.find("config") != std::string::npos ||
            import.find("control") != std::string::npos) {
            return true;
        }
    }
    return false;
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc != 2) {
        std::cerr << "Rust TSF PoC DLL argument required\n";
        return 1;
    }
    const std::filesystem::path dllPath = argv[1];
    std::ifstream input(dllPath, std::ios::binary);
    if (!input) {
        std::cerr << "could not open Rust TSF PoC DLL\n";
        return 1;
    }
    const std::vector<uint8_t> image((std::istreambuf_iterator<char>(input)),
                                     std::istreambuf_iterator<char>());
    try {
        const PeAudit audit = parsePe(image);
        if (audit.machine != 0x014c && audit.machine != 0x8664 && audit.machine != 0xaa64) {
            std::cerr << "unsupported Rust TSF PoC machine type\n";
            return 1;
        }
        if (audit.majorOperatingSystem > 10 || audit.majorSubsystem > 10) {
            std::cerr << "Rust TSF PoC requires a future unsupported Windows version\n";
            return 1;
        }
        if (image.size() > 2 * 1024 * 1024) {
            std::cerr << "Rust TSF PoC DLL unexpectedly exceeds binary-size smoke budget\n";
            return 1;
        }
        if (hasImport(audit, "winhttp.dll") || hasImport(audit, "wininet.dll") ||
            hasImport(audit, "ws2_32.dll") || hasImport(audit, "urlmon.dll")) {
            std::cerr << "Rust TSF PoC must not import network/web runtime libraries\n";
            return 1;
        }
        if (hasForbiddenProductImport(audit)) {
            std::cerr << "Rust TSF PoC must not import product engine/config/package/control DLLs\n";
            return 1;
        }
        if ((audit.dllCharacteristics & 0x0040) == 0 || (audit.dllCharacteristics & 0x0100) == 0) {
            std::cerr << "Rust TSF PoC should keep ASLR and NX compatible PE flags\n";
            return 1;
        }
    } catch (const std::exception& error) {
        std::cerr << "Rust TSF PoC artifact audit failed: " << error.what() << '\n';
        return 1;
    }
    return 0;
}
