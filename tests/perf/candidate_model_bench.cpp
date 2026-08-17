#include "candidate_model.h"

#include <chrono>
#include <iostream>

int main() {
    using namespace fcitx::windows::candidate;
    CandidateModel model;
    Snapshot snapshot{1, 2, 3, 1, "ni", {}, {}, {}, 0, 0, 10,
                      Visibility::composition};
    for (std::uint64_t index = 0; index < 10; ++index) {
        snapshot.candidates.push_back(
            Item{index + 1, std::to_string(index + 1), "candidate", "comment"});
    }
    constexpr std::uint64_t iterations = 100'000;
    const auto begin = std::chrono::steady_clock::now();
    for (std::uint64_t iteration = 0; iteration < iterations; ++iteration) {
        snapshot.revision = iteration + 1;
        if (model.apply(snapshot) != ApplyResult::applied) return 1;
    }
    const auto elapsed = std::chrono::steady_clock::now() - begin;
    const auto nanoseconds =
        std::chrono::duration_cast<std::chrono::nanoseconds>(elapsed).count();
    std::cout << "candidate-model-ns/op="
              << static_cast<double>(nanoseconds) / static_cast<double>(iterations) << '\n';
    return 0;
}
