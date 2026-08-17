#include "candidate_model.h"

#include <algorithm>
#include <utility>

namespace fcitx::windows::candidate {
namespace {

bool validText(const std::string& value) noexcept {
    return value.size() <= kMaxCandidateTextUtf8 &&
           value.find('\0') == std::string::npos;
}

} // namespace

bool validate(const Snapshot& snapshot) noexcept {
    if (snapshot.engineEpoch == 0 || snapshot.contextId == 0 || snapshot.revision == 0 ||
        snapshot.candidates.size() > kMaxCandidates || !validText(snapshot.preedit) ||
        !validText(snapshot.auxiliaryUp) || !validText(snapshot.auxiliaryDown)) {
        return false;
    }
    if (snapshot.selected && *snapshot.selected >= snapshot.candidates.size()) return false;
    if (snapshot.total < snapshot.candidates.size()) return false;
    if (snapshot.visibility == Visibility::hidden && !snapshot.candidates.empty()) {
        return false;
    }
    if (snapshot.visibility != Visibility::hidden && snapshot.compositionId == 0) return false;
    return std::all_of(snapshot.candidates.begin(), snapshot.candidates.end(),
                       [](const Item& item) {
                           return item.id != 0 && validText(item.label) &&
                                  validText(item.text) && validText(item.comment);
                       });
}

ApplyResult CandidateModel::apply(Snapshot snapshot) {
    if (!validate(snapshot)) return ApplyResult::invalid;
    if (current_) {
        if (snapshot.engineEpoch < current_->engineEpoch) return ApplyResult::stale;
        if (snapshot.engineEpoch == current_->engineEpoch) {
            if (snapshot.contextId == current_->contextId &&
                snapshot.revision < current_->revision) return ApplyResult::stale;
            if (snapshot.contextId == current_->contextId &&
                snapshot.revision == current_->revision) {
                return snapshot == *current_ ? ApplyResult::duplicate : ApplyResult::stale;
            }
            if (snapshot.compositionId != 0 && current_->compositionId != 0 &&
                snapshot.compositionId < current_->compositionId) {
                return ApplyResult::stale;
            }
        }
    }
    current_ = std::move(snapshot);
    return ApplyResult::applied;
}

void CandidateModel::reset() noexcept { current_.reset(); }

const std::optional<Snapshot>& CandidateModel::current() const noexcept { return current_; }

} // namespace fcitx::windows::candidate
