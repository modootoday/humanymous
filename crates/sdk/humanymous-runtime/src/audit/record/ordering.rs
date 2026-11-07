// Copyright 2022 The Go Authors. All rights reserved.
// Adapted from Go 1.26.3 src/sort/zsortfunc.go and sort.go for the frozen audit
// format. BSD-3-Clause; full notice is in ../../LICENSE-Go.txt.
//
// Rust's stable and unstable sorts both differ on equal IDs. Historical record
// hashes include Go sort.Slice's exact equal-key permutation, so this narrow
// compatibility implementation must not be replaced by an ordinary Rust sort.
use super::Signal;

pub(super) fn sort_signals(data: &mut [&Signal]) {
    let length = data.len();
    pdqsort(
        data,
        0,
        length,
        (usize::BITS - length.leading_zeros()) as usize,
    );
}
fn less(data: &[&Signal], left: usize, right: usize) -> bool {
    data[left].id < data[right].id
}
fn insertion(data: &mut [&Signal], a: usize, b: usize) {
    for i in a + 1..b {
        let mut j = i;
        while j > a && less(data, j, j - 1) {
            data.swap(j, j - 1);
            j -= 1;
        }
    }
}
fn sift_down(data: &mut [&Signal], mut root: usize, hi: usize, first: usize) {
    loop {
        let mut child = 2 * root + 1;
        if child >= hi {
            break;
        }
        if child + 1 < hi && less(data, first + child, first + child + 1) {
            child += 1;
        }
        if !less(data, first + root, first + child) {
            return;
        }
        data.swap(first + root, first + child);
        root = child;
    }
}
fn heap_sort(data: &mut [&Signal], a: usize, b: usize) {
    let hi = b - a;
    if hi == 0 {
        return;
    }
    for i in (0..=(hi - 1) / 2).rev() {
        sift_down(data, i, hi, a);
    }
    for i in (0..hi).rev() {
        data.swap(a, a + i);
        sift_down(data, 0, i, a);
    }
}
fn pdqsort(data: &mut [&Signal], mut a: usize, mut b: usize, mut limit: usize) {
    let (mut balanced, mut partitioned) = (true, true);
    loop {
        let length = b - a;
        if length <= 12 {
            insertion(data, a, b);
            return;
        }
        if limit == 0 {
            heap_sort(data, a, b);
            return;
        }
        if !balanced {
            break_patterns(data, a, b);
            limit -= 1;
        }
        let (mut pivot, mut hint) = choose_pivot(data, a, b);
        if hint == 2 {
            data[a..b].reverse();
            pivot = (b - 1) - (pivot - a);
            hint = 1;
        }
        if balanced && partitioned && hint == 1 && partial_insertion(data, a, b) {
            return;
        }
        if a > 0 && !less(data, a - 1, pivot) {
            a = partition_equal(data, a, b, pivot);
            continue;
        }
        let (mid, already) = partition(data, a, b, pivot);
        partitioned = already;
        let (left, right) = (mid - a, b - mid);
        let threshold = length / 8;
        if left < right {
            balanced = left >= threshold;
            pdqsort(data, a, mid, limit);
            a = mid + 1;
        } else {
            balanced = right >= threshold;
            pdqsort(data, mid + 1, b, limit);
            b = mid;
        }
    }
}
fn partition(data: &mut [&Signal], a: usize, b: usize, pivot: usize) -> (usize, bool) {
    data.swap(a, pivot);
    let (mut i, mut j) = (a + 1, b - 1);
    while i <= j && less(data, i, a) {
        i += 1;
    }
    while i <= j && !less(data, j, a) {
        j -= 1;
    }
    if i > j {
        data.swap(j, a);
        return (j, true);
    }
    data.swap(i, j);
    i += 1;
    j -= 1;
    loop {
        while i <= j && less(data, i, a) {
            i += 1;
        }
        while i <= j && !less(data, j, a) {
            j -= 1;
        }
        if i > j {
            break;
        }
        data.swap(i, j);
        i += 1;
        j -= 1;
    }
    data.swap(j, a);
    (j, false)
}
fn partition_equal(data: &mut [&Signal], a: usize, b: usize, pivot: usize) -> usize {
    data.swap(a, pivot);
    let (mut i, mut j) = (a + 1, b - 1);
    loop {
        while i <= j && !less(data, a, i) {
            i += 1;
        }
        while i <= j && less(data, a, j) {
            j -= 1;
        }
        if i > j {
            break;
        }
        data.swap(i, j);
        i += 1;
        j -= 1;
    }
    i
}
fn partial_insertion(data: &mut [&Signal], a: usize, b: usize) -> bool {
    let mut i = a + 1;
    for _ in 0..5 {
        while i < b && !less(data, i, i - 1) {
            i += 1;
        }
        if i == b {
            return true;
        }
        if b - a < 50 {
            return false;
        }
        data.swap(i, i - 1);
        if i - a >= 2 {
            let mut j = i - 1;
            while j >= 1 {
                if !less(data, j, j - 1) {
                    break;
                }
                data.swap(j, j - 1);
                j -= 1;
            }
        }
        if b - i >= 2 {
            for j in i + 1..b {
                if !less(data, j, j - 1) {
                    break;
                }
                data.swap(j, j - 1);
            }
        }
    }
    false
}
fn break_patterns(data: &mut [&Signal], a: usize, b: usize) {
    let length = b - a;
    if length >= 8 {
        let mut random = length as u64;
        let modulus = 1u64 << (usize::BITS - length.leading_zeros());
        for index in a + (length / 4) * 2 - 1..=a + (length / 4) * 2 + 1 {
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            let mut other = (random & (modulus - 1)) as usize;
            if other >= length {
                other -= length;
            }
            data.swap(index, a + other);
        }
    }
}
fn choose_pivot(data: &[&Signal], a: usize, b: usize) -> (usize, u8) {
    let length = b - a;
    let mut swaps = 0;
    let (mut i, mut j, mut k) = (a + length / 4, a + length / 4 * 2, a + length / 4 * 3);
    if length >= 8 {
        if length >= 50 {
            i = median(data, i - 1, i, i + 1, &mut swaps);
            j = median(data, j - 1, j, j + 1, &mut swaps);
            k = median(data, k - 1, k, k + 1, &mut swaps);
        }
        j = median(data, i, j, k, &mut swaps);
    }
    (
        j,
        match swaps {
            0 => 1,
            12 => 2,
            _ => 0,
        },
    )
}
fn order2(data: &[&Signal], a: usize, b: usize, swaps: &mut u8) -> (usize, usize) {
    if less(data, b, a) {
        *swaps += 1;
        (b, a)
    } else {
        (a, b)
    }
}
fn median(data: &[&Signal], a: usize, b: usize, c: usize, swaps: &mut u8) -> usize {
    let (a, b) = order2(data, a, b, swaps);
    let (b, _c) = order2(data, b, c, swaps);
    let (_a, b) = order2(data, a, b, swaps);
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equal_ids_retain_the_historical_go_permutation() {
        let observations = (0..13)
            .map(|index| Signal {
                id: ["a", "m", "a", "é", "e\u{301}", "z"][index % 6].into(),
                conf: index as f64,
                ..Default::default()
            })
            .collect::<Vec<_>>();
        let mut refs = observations.iter().collect::<Vec<_>>();
        sort_signals(&mut refs);
        assert!(refs.windows(2).all(|pair| pair[0].id <= pair[1].id));
        assert_ne!(
            refs.iter()
                .filter(|s| s.id == "a")
                .map(|s| s.conf as usize)
                .collect::<Vec<_>>(),
            vec![0, 2, 6, 8, 12]
        );
    }

    #[test]
    fn adversarial_patterns_and_partition_boundaries_match_go() {
        #[derive(serde::Deserialize)]
        struct Case {
            ids: Vec<String>,
            expected: Vec<usize>,
        }
        #[derive(serde::Deserialize)]
        struct Corpus {
            orderings: Vec<Case>,
        }
        let corpus: Corpus =
            serde_json::from_str(include_str!("../../../tests/fixtures/audit/records.json"))
                .unwrap();
        assert_eq!(corpus.orderings.len(), 104);
        for case in corpus.orderings {
            let observations = case
                .ids
                .into_iter()
                .enumerate()
                .map(|(index, id)| Signal {
                    id,
                    conf: index as f64,
                    ..Default::default()
                })
                .collect::<Vec<_>>();
            let mut refs = observations.iter().collect::<Vec<_>>();
            sort_signals(&mut refs);
            assert_eq!(
                refs.iter().map(|s| s.conf as usize).collect::<Vec<_>>(),
                case.expected
            );
        }
    }
}
