//! Bounded guard epochs. Browser owners must supply real baseline and hook coverage.
use humanymous_core::signals::report::Guard;
mod native;
pub use native::native_source_marker;

/// Measured own-property counts, not a claim about the contents of prototypes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrototypeCounts {
    object: u16,
    array: u16,
}
impl PrototypeCounts {
    /// Reject excessive enumerations rather than silently clipping a baseline.
    pub fn new(object: usize, array: usize) -> Result<Self, &'static str> {
        if object > 16_384 || array > 16_384 {
            return Err("guard prototype observation budget");
        }
        Ok(Self {
            object: object as u16,
            array: array as u16,
        })
    }
}
/// Startup observations supplied by the checked browser acquisition boundary.
#[derive(Clone, Copy, Debug)]
pub struct NativeBaseline {
    /// Whether fetch passed the original captured-native-toString check.
    pub fetch: bool,
    /// Whether XMLHttpRequest passed that check.
    pub xhr: bool,
    /// Whether JSON.stringify passed that check.
    pub stringify: bool,
    /// Whether JSON.parse passed that check.
    pub parse: bool,
    /// Whether console.log exists, is callable and passed the native check.
    pub console_log: bool,
}
/// A hook category doubles as the only accepted counter event; no payload is stored.
#[derive(Clone, Copy, Debug)]
pub enum Hook {
    /// Observed invocation through the installed global eval reference.
    Eval,
    /// Observed invocation/construction through the installed global Function reference.
    Function,
    /// Observed unmarked script node added by the owned mutation observer.
    Script,
}
/// One epoch's state; this type itself neither observes nor modifies a browser.
pub struct Epoch {
    baseline: PrototypeCounts,
    report: Guard,
    coverage: [bool; 3],
    frozen: Option<Guard>,
    poisoned: bool,
}
impl Epoch {
    /// Begin from caller-supplied measured baselines, with every dynamic hook uncovered.
    pub fn new(baseline: PrototypeCounts, native: NativeBaseline) -> Self {
        Self {
            baseline,
            report: Guard {
                native_hooked: !(native.fetch && native.xhr && native.stringify && native.parse),
                console_disabled: !native.console_log,
                ..Default::default()
            },
            coverage: [false; 3],
            frozen: None,
            poisoned: false,
        }
    }
    /// The browser owner calls this only after installing the corresponding real hook.
    pub fn cover(&mut self, hook: Hook) {
        if self.frozen.is_none() && !self.poisoned {
            self.coverage[index(hook)] = true;
        }
    }
    /// Count an event without receiving its code, node contents or caller arguments.
    pub fn record(&mut self, hook: Hook) -> Result<(), &'static str> {
        if self.poisoned {
            return Err("guard epoch invalid");
        }
        if self.frozen.is_some() {
            return Ok(());
        }
        let value = match hook {
            Hook::Eval => &mut self.report.eval_used,
            Hook::Function => &mut self.report.func_ctor,
            Hook::Script => &mut self.report.script_injected,
        };
        if *value >= 9_007_199_254_740_991 {
            self.poisoned = true;
            return Err("guard counter precision budget");
        }
        *value += 1;
        Ok(())
    }
    /// Read measured current counts; absent coverage keeps reported=false.
    pub fn snapshot(&self, current: PrototypeCounts) -> Result<Guard, &'static str> {
        if self.poisoned {
            return Err("guard epoch invalid");
        }
        if let Some(report) = &self.frozen {
            return Ok(report.clone());
        }
        let mut report = self.report.clone();
        report.reported = self.coverage.iter().all(|covered| *covered);
        report.proto_polluted = current != self.baseline;
        Ok(report)
    }
    /// Freeze once; later callbacks/coverage changes cannot rewrite this snapshot.
    pub fn stop(&mut self, current: PrototypeCounts) -> Result<Guard, &'static str> {
        let report = self.snapshot(current)?;
        self.frozen = Some(report.clone());
        Ok(report)
    }
}
fn index(hook: Hook) -> usize {
    match hook {
        Hook::Eval => 0,
        Hook::Function => 1,
        Hook::Script => 2,
    }
}

#[cfg(test)]
mod tests;
