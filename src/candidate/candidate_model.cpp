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

    if (engineEpoch_ != 0 && snapshot.engineEpoch < engineEpoch_)
        return ApplyResult::stale;
    if (engineEpoch_ == 0 || snapshot.engineEpoch > engineEpoch_) {
        engineEpoch_ = snapshot.engineEpoch;
        current_.reset();
        freshness_.clear();
        freshnessOrder_.clear();
    }

    if (const auto found = freshness_.find(snapshot.contextId);
        found != freshness_.end()) {
        const auto& freshness = found->second;
        if (snapshot.compositionId == freshness.compositionId &&
            snapshot.revision == freshness.revision) {
            return current_ && snapshot == *current_ ? ApplyResult::duplicate
                                                     : ApplyResult::stale;
        }
        if (snapshot.compositionId == freshness.compositionId) {
            if (snapshot.revision < freshness.revision) return ApplyResult::stale;
        } else if (snapshot.compositionId == 0) {
            if (snapshot.revision < freshness.revision) return ApplyResult::stale;
        } else if (snapshot.compositionId <= freshness.latestCompositionId) {
            return ApplyResult::stale;
        }
    }

    rememberContext(snapshot.contextId, snapshot);
    current_ = std::move(snapshot);
    return ApplyResult::applied;
}

void CandidateModel::rememberContext(std::uint64_t contextId, const Snapshot& snapshot) {
    auto [iterator, inserted] = freshness_.try_emplace(contextId);
    if (inserted) {
        freshnessOrder_.push_back(contextId);
        while (freshness_.size() > kMaxTrackedContexts && !freshnessOrder_.empty()) {
            const auto evicted = freshnessOrder_.front();
            freshnessOrder_.pop_front();
            if (evicted != contextId)
                freshness_.erase(evicted);
        }
    }
    auto& freshness = iterator->second;
    freshness.compositionId = snapshot.compositionId;
    if (snapshot.compositionId != 0 &&
        snapshot.compositionId > freshness.latestCompositionId) {
        freshness.latestCompositionId = snapshot.compositionId;
    }
    freshness.revision = snapshot.revision;
}

void CandidateModel::reset() noexcept {
    current_.reset();
    engineEpoch_ = 0;
    freshness_.clear();
    freshnessOrder_.clear();
}

const std::optional<Snapshot>& CandidateModel::current() const noexcept { return current_; }

} // namespace fcitx::windows::candidate
