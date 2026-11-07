use super::{Snapshot, add};
use humanymous_core::signals::observation::{Signal, Verdict};
use serde_json::json;

pub(super) fn collect(s: &Snapshot, out: &mut Vec<Signal>) {
    if !s.webgl_renderer.is_empty() {
        let low = humanymous_core::signals::wire::simple_lowercase(&format!(
            "{} {}",
            s.webgl_renderer, s.webgl_vendor
        ));
        if ["swiftshader", "llvmpipe", "mesa offscreen"]
            .iter()
            .any(|m| low.contains(m))
        {
            add(
                out,
                "l2.webgl.swiftshader",
                json!(s.webgl_renderer),
                Verdict::Bot,
                1.0,
                &format!("software renderer: {}", s.webgl_renderer),
            );
        }
    }
    if let Some(memory) = s.memory
        && memory > 0.0
        && ![0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0].contains(&memory)
    {
        add(
            out,
            "l2.hardware.devicememory",
            json!(memory),
            Verdict::Suspicious,
            0.8,
            "deviceMemory not power-of-two (manual override)",
        );
    }
    if let Some((height, available)) = s.screen
        && height != 0
        && height == available
    {
        add(
            out,
            "l2.screen.metrics",
            json!({"height":height,"availHeight":available}),
            Verdict::Suspicious,
            0.5,
            "availHeight==height (no taskbar)",
        );
    }
}
