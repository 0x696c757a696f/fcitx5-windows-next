use fcitx5_measurement_core::{run_fixture, Architecture};

fn main() {
    let architecture = if cfg!(target_pointer_width = "32") {
        Architecture::X86
    } else {
        Architecture::X64
    };
    let report = run_fixture(architecture);
    println!("{{\"schema_version\":{},\"architecture\":\"{}\",\"calibration\":\"{}\",\"privacy_safe\":{},\"measurements\":[{}]}}",
        report.schema_version, report.architecture, report.calibration, report.privacy_safe,
        report.measurements.iter().map(|sample| format!(
            "{{\"surface\":\"{:?}\",\"workload\":\"{}\",\"latency_us\":{},\"memory_bytes\":{},\"operations\":{}}}",
            sample.surface, sample.workload, sample.latency_us, sample.memory_bytes, sample.operations
        )).collect::<Vec<_>>().join(","));
}
