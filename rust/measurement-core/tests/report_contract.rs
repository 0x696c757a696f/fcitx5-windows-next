#![forbid(unsafe_code)]

use fcitx5_measurement_core::{run_fixture, Architecture, MeasurementSurface};

#[test]
fn report_is_bounded_privacy_safe_and_separates_heavy_plugins() {
    let report = run_fixture(Architecture::X64);
    assert_eq!(report.schema_version, 1);
    assert_eq!(report.measurements.len(), 7);
    assert!(report.measurements.iter().all(|sample| {
        sample.surface != MeasurementSurface::HeavyPluginActivation
            || matches!(sample.workload.as_str(), "rime" | "mozc" | "lua")
    }));
    assert!(report.measurements.iter().all(|sample| {
        sample.latency_us > 0
            && sample.memory_bytes > 0
            && sample.operations > 0
            && sample.workload.len() <= 16
    }));
    assert!(report.privacy_safe);
}
