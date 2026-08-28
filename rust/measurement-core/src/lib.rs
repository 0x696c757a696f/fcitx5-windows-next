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
    fn architecture_changes_only_fixture_scale() {
        let x64 = run_fixture(Architecture::X64);
        let x86 = run_fixture(Architecture::X86);
        assert_eq!(x64.measurements.len(), x86.measurements.len());
        assert!(x86.measurements[0].latency_us > x64.measurements[0].latency_us);
        assert_eq!(x64.calibration, "initial SLO; pending real calibration");
    }
}
