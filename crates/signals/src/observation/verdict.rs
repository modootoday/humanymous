use super::Verdict;

/// Frozen verdict multiplier.
pub fn severity(verdict: &Verdict) -> f64 {
    match verdict {
        Verdict::Bot => 1.0,
        Verdict::Suspicious => 0.5,
        _ => 0.0,
    }
}

/// Apply the existing signal-weight and confidence policy, including its clamp order.
pub fn compute_score(weight: f64, confidence: f64, verdict: &Verdict) -> f64 {
    let score = weight * severity(verdict) * confidence.clamp(0.0, 1.0);
    if score < 0.0 {
        0.0
    } else if score > weight {
        weight
    } else {
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_confidence_and_preserves_severity() {
        assert_eq!(compute_score(20.0, 2.0, &Verdict::Bot), 20.0);
        assert_eq!(compute_score(20.0, -1.0, &Verdict::Bot), 0.0);
        assert_eq!(compute_score(20.0, 0.8, &Verdict::Suspicious), 8.0);
        assert_eq!(compute_score(20.0, 1.0, &Verdict::Unknown), 0.0);
        assert_eq!(
            compute_score(20.0, 1.0, &Verdict::Other("future".into())),
            0.0
        );
        assert!(compute_score(20.0, f64::NAN, &Verdict::Bot).is_nan());
    }
}
