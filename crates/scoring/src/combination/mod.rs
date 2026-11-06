use crate::{
    policy::Policy,
    trace::{DedupTrace, LayerTrace},
};
use humanymous_signals::{observation::Layer, report::Contributor};
use std::collections::HashMap;

pub(crate) struct Contribution {
    pub id: String,
    pub layer: Layer,
    pub group: &'static str,
    pub score: f64,
}
pub(crate) struct Combination {
    pub risk: f64,
    pub layers: Option<Vec<LayerTrace>>,
    pub drops: Option<Vec<DedupTrace>>,
}

pub(crate) fn combine(input: &[Contribution], policy: &Policy, traced: bool) -> Combination {
    let mut groups: HashMap<(&str, &str), usize> = HashMap::new();
    let mut best: Vec<usize> = Vec::new();
    let mut members: Vec<Vec<usize>> = Vec::new();
    let mut loose = Vec::new();
    for (index, contribution) in input.iter().enumerate() {
        if contribution.group.is_empty() {
            loose.push(index);
            continue;
        }
        let key = (contribution.layer.as_str(), contribution.group);
        if let Some(&group) = groups.get(&key) {
            if contribution.score > input[best[group]].score {
                best[group] = index;
            }
            if traced {
                members[group].push(index);
            }
        } else {
            groups.insert(key, best.len());
            best.push(index);
            if traced {
                members.push(vec![index]);
            }
        }
    }
    let mut drops = Vec::new();
    if traced {
        for (group, &index) in best.iter().enumerate() {
            let kept = &input[index];
            for &member in &members[group] {
                let dropped = &input[member];
                if dropped.id != kept.id {
                    drops.push(DedupTrace {
                        layer: kept.layer.as_str().into(),
                        group: kept.group.into(),
                        kept_id: kept.id.clone(),
                        dropped_id: dropped.id.clone(),
                        kept_score: kept.score,
                        dropped_score: dropped.score,
                    });
                }
            }
        }
    }
    // First-seen group/layer order is the existing Go trace order. Both public
    // score paths share it rather than allowing map iteration to change math.
    let mut layer_indices: HashMap<&str, usize> = HashMap::new();
    let mut probabilities: Vec<(&str, f64)> = Vec::new();
    for &index in best.iter().chain(&loose) {
        let contribution = &input[index];
        let layer = contribution.layer.as_str();
        let position = *layer_indices.entry(layer).or_insert_with(|| {
            probabilities.push((layer, 0.0));
            probabilities.len() - 1
        });
        let probability = (contribution.score / 100.0).clamp(0.0, 1.0);
        let current = probabilities[position].1;
        probabilities[position].1 = current + probability - current * probability;
    }
    let cap = (policy.layer_cap / 100.0).clamp(0.0, 1.0);
    let mut product = 1.0;
    let mut layers = Vec::new();
    for (layer, raw) in probabilities {
        let capped = if raw > cap { cap } else { raw };
        product *= 1.0 - capped.clamp(0.0, 1.0);
        if traced {
            layers.push(LayerTrace {
                layer: layer.into(),
                raw_prob: raw,
                capped_prob: capped,
                cap_hit: raw > cap,
            });
        }
    }
    Combination {
        risk: 100.0 * (1.0 - product),
        layers: (!layers.is_empty()).then_some(layers),
        drops: (!drops.is_empty()).then_some(drops),
    }
}

pub(crate) fn top_contributors(input: &[Contribution], limit: usize) -> Vec<Contributor> {
    let mut ordered: Vec<_> = input.iter().collect();
    ordered.sort_by(|first, second| {
        second
            .score
            .partial_cmp(&first.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ordered
        .into_iter()
        .take(limit)
        .take_while(|contribution| contribution.score > 0.0)
        .map(|contribution| Contributor {
            id: contribution.id.clone(),
            score: contribution.score,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn item(id: &str, layer: Layer, group: &'static str, score: f64) -> Contribution {
        Contribution {
            id: id.into(),
            layer,
            group,
            score,
        }
    }
    #[test]
    fn trace_and_plain_share_dedup_caps_and_stable_ties() {
        let input = vec![
            item("first", Layer::Static, "plugins", 25.0),
            item("best", Layer::Static, "plugins", 40.0),
            item("third", Layer::Fingerprint, "canvas", 55.0),
        ];
        let traced = combine(&input, &Policy::default(), true);
        assert_eq!(traced.risk, combine(&input, &Policy::default(), false).risk);
        assert_eq!(traced.drops.unwrap()[0].kept_id, "best");
        let input = vec![
            item("a", Layer::Static, "", 55.0),
            item("b", Layer::Static, "", 55.0),
            item("c", Layer::Network, "", 30.0),
        ];
        let traced = combine(&input, &Policy::default(), true);
        assert!(traced.layers.unwrap()[0].cap_hit);
        assert_eq!(
            top_contributors(&input, 2)
                .iter()
                .map(|value| value.id.as_str())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
    }
}
