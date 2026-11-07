use super::{Getter, Snapshot, add};
use humanymous_signals::observation::{Signal, Verdict};
use serde_json::json;
pub(super) fn collect(s: &Snapshot, out: &mut Vec<Signal>) {
    if matches!(s.getter, Getter::Patched) {
        add(
            out,
            "l3.integrity.native_tostring",
            json!(false),
            Verdict::Bot,
            1.0,
            "webdriver getter is not native (patched)",
        );
    }
    if let Some((top, inner)) = s.iframe
        && top != inner
    {
        add(
            out,
            "l3.integrity.iframe_recheck",
            json!(inner),
            Verdict::Bot,
            1.0,
            "iframe realm disagrees on webdriver",
        );
    }
}
