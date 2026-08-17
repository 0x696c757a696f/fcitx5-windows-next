#include "candidate_model.h"

#include <Windows.h>

#include <cstdint>
#include <iostream>

int main() {
    using namespace fcitx::windows::candidate;
    constexpr std::uint64_t iterations = 10'000;
    CandidateModel model;
    LARGE_INTEGER frequency{};
    LARGE_INTEGER start{};
    LARGE_INTEGER finish{};
    QueryPerformanceFrequency(&frequency);
    QueryPerformanceCounter(&start);
    for (std::uint64_t context = 1; context <= iterations; ++context) {
        Snapshot current;
        current.engineEpoch = 1;
        current.contextId = context;
        current.compositionId = context;
        current.revision = 1;
        current.preedit = "n";
        current.visibility = Visibility::composition;
        current.candidates.push_back({context, "1", "ni", ""});
        current.total = 1;
        if (model.apply(std::move(current)) != ApplyResult::applied) return 1;

        if (context > 1) {
            Snapshot stale;
            stale.engineEpoch = 1;
            stale.contextId = context - 1;
            stale.compositionId = context - 1;
            stale.revision = 2;
            stale.preedit = "stale";
            stale.visibility = Visibility::composition;
            if (model.apply(std::move(stale)) != ApplyResult::stale) return 1;
        }
    }
    QueryPerformanceCounter(&finish);
    const auto& current = model.current();
    if (!current || current->contextId != iterations || current->preedit != "n") return 1;
    const double elapsedMilliseconds =
        static_cast<double>(finish.QuadPart - start.QuadPart) * 1000.0 /
        static_cast<double>(frequency.QuadPart);
    std::cout << "focus-context-churn iterations=" << iterations
              << " elapsed-ms=" << elapsedMilliseconds
              << " ns-per-switch=" << elapsedMilliseconds * 1'000'000.0 / iterations
              << '\n';
    return 0;
}
