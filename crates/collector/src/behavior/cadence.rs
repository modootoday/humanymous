use super::statistics::variance;
pub(super) fn reduce(times: &[f64]) -> (i64, f64, f64) {
    if times.len() < 4 {
        return (0, 0.0, 0.0);
    }
    let gaps: Vec<_> = times.windows(2).map(|p| p[1] - p[0]).collect();
    let mut intra = Vec::new();
    let mut long = 0;
    let mut reaction = f64::INFINITY;
    for (index, gap) in gaps.iter().enumerate() {
        if *gap > 1500.0 {
            long += 1;
            if let Some(next) = gaps.get(index + 1) {
                reaction = reaction.min(*next);
            }
        } else {
            intra.push(*gap);
        }
    }
    (
        long,
        variance(&intra),
        if reaction.is_infinite() {
            0.0
        } else {
            reaction
        },
    )
}
