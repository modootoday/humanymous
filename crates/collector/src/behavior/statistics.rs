pub(super) fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}
pub(super) fn variance(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let average = mean(values);
    values
        .iter()
        .map(|v| (v - average) * (v - average))
        .sum::<f64>()
        / values.len() as f64
}
