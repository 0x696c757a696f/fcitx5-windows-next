#![forbid(unsafe_code)]

//! Deterministic, privacy-safe low-resource measurement fixtures.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Architecture {
    X64,
    X86,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeasurementSurface {
    Core,
    TsfShim,
    CandidateUi,
    HeavyPluginActivation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sample {
    pub surface: MeasurementSurface,
    pub workload: String,
    pub latency_us: u64,
    pub memory_bytes: u64,
    pub operations: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Report {
    pub schema_version: u32,
    pub architecture: &'static str,
    pub calibration: &'static str,
    pub privacy_safe: bool,
    pub measurements: Vec<Sample>,
}

/// Formats the bounded key roundtrip result consumed by the benchmark script.
pub fn format_key_roundtrip_result(architecture_bits: usize, samples: &[f64]) -> Option<String> {
    if samples.is_empty() {
        return None;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let percentile = |numerator: usize| sorted[((sorted.len() - 1) * numerator) / 100];
    Some(format!(
        "{{\"benchmark\":\"key_roundtrip\",\"architecture_bits\":{},\"samples\":{},\"p50_us\":{},\"p95_us\":{},\"p99_us\":{},\"max_us\":{}}}",
        architecture_bits,
        sorted.len(),
        percentile(50),
        percentile(95),
        percentile(99),
        sorted[sorted.len() - 1]
    ))
}

struct FakeClock(u64);

impl FakeClock {
    fn measure(&mut self, work: u32) -> (u64, u64) {
        let start = self.0;
        for step in 0..work {
            self.0 = self.0.wrapping_add(17 + u64::from(step % 11));
        }
        (self.0 - start, u64::from(work) * 64)
    }
}

pub fn run_fixture(architecture: Architecture) -> Report {
    let mut clock = FakeClock(1_000);
    let width = match architecture {
        Architecture::X64 => 1,
        Architecture::X86 => 2,
    };
    let mut sample = |surface, workload: &'static str, work| {
        let (latency_us, memory_bytes) = clock.measure(work * width);
        Sample {
            surface,
            workload: workload.to_owned(),
            latency_us,
            memory_bytes,
            operations: work,
        }
    };
    Report {
        schema_version: 1,
        architecture: match architecture {
            Architecture::X64 => "x64",
            Architecture::X86 => "x86",
        },
        calibration: "initial SLO; pending real calibration",
        privacy_safe: true,
        measurements: vec![
            sample(MeasurementSurface::Core, "startup", 32),
            sample(MeasurementSurface::Core, "key_roundtrip", 48),
            sample(MeasurementSurface::TsfShim, "activation", 24),
            sample(MeasurementSurface::CandidateUi, "layout", 40),
            sample(MeasurementSurface::HeavyPluginActivation, "rime", 80),
            sample(MeasurementSurface::HeavyPluginActivation, "mozc", 88),
            sample(MeasurementSurface::HeavyPluginActivation, "lua", 56),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_roundtrip_result_uses_compatible_json_fields() {
        let result = format_key_roundtrip_result(64, &[1.0, 2.0, 3.0, 4.0]);

        assert_eq!(
            result,
            Some(
                "{\"benchmark\":\"key_roundtrip\",\"architecture_bits\":64,\"samples\":4,\"p50_us\":2,\"p95_us\":3,\"p99_us\":3,\"max_us\":4}".to_owned()
            )
        );
    }

    #[test]
    fn architecture_changes_only_fixture_scale() {
        let x64 = run_fixture(Architecture::X64);
        let x86 = run_fixture(Architecture::X86);
        assert_eq!(x64.measurements.len(), x86.measurements.len());
        assert!(x86.measurements[0].latency_us > x64.measurements[0].latency_us);
        assert_eq!(x64.calibration, "initial SLO; pending real calibration");
    }
}
