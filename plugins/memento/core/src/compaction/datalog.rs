//! The retention fixed point is evaluated by Crepe, not by explanation traversal.
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crepe::crepe;

use super::{MAX_REASONS, Reason};

type RootFact = (u64, &'static str);
type DependencyFact = (u64, u64, &'static str);

#[derive(Debug, Clone, Default)]
pub(super) struct Groups {
    pub(super) uses: BTreeSet<(u64, usize, &'static str)>,
    pub(super) members: BTreeSet<(usize, u64)>,
}

crepe! {
    @input
    struct Root(u64, &'static str);

    @input
    struct Dependency(u64, u64, &'static str);

    @input
    struct UsesGroup(u64, usize, &'static str);

    @input
    struct Member(usize, u64);

    struct ActiveGroup(usize);

    @output
    struct Keep(u64);

    Keep(sequence) <- Root(sequence, _);
    Keep(to) <- Keep(from), Dependency(from, to, _);
    ActiveGroup(group) <- Keep(from), UsesGroup(from, group, _);
    Keep(to) <- ActiveGroup(group), Member(group, to);
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Evaluation {
    pub(super) sequences: BTreeSet<u64>,
    pub(super) reasons: Vec<Reason>,
}

pub(super) fn evaluate(
    roots: impl IntoIterator<Item = RootFact>,
    dependencies: impl IntoIterator<Item = DependencyFact>,
    groups: Groups,
) -> Evaluation {
    // Canonical facts make explanation tie-breaking independent of input order
    // and duplicate facts. Crepe's output itself has set semantics.
    let roots: BTreeSet<_> = roots.into_iter().collect();
    let dependencies: BTreeSet<_> = dependencies.into_iter().collect();
    let mut runtime = Crepe::new();
    runtime.extend(roots.iter().map(|&(sequence, rule)| Root(sequence, rule)));
    runtime.extend(
        dependencies
            .iter()
            .map(|&(from, to, rule)| Dependency(from, to, rule)),
    );
    runtime.extend(
        groups
            .uses
            .iter()
            .map(|&(from, group, rule)| UsesGroup(from, group, rule)),
    );
    runtime.extend(groups.members.iter().map(|&(group, to)| Member(group, to)));
    let (keep,) = runtime.run();
    let sequences: BTreeSet<_> = keep.into_iter().map(|Keep(sequence)| sequence).collect();
    let reasons = explain(&roots, &dependencies, &groups, &sequences);
    Evaluation { sequences, reasons }
}

/// Select a bounded, deterministic witness forest for the already evaluated set.
/// Roots take precedence; sorted breadth-first traversal then selects a shortest
/// derivation, breaking ties by root sequence and dependency sequence/rule.
/// A dependency is explained only after its parent, so cycles cannot support
/// themselves. This traversal never decides which rows are retained or removed.
fn explain(
    roots: &BTreeSet<RootFact>,
    dependencies: &BTreeSet<DependencyFact>,
    groups: &Groups,
    retained: &BTreeSet<u64>,
) -> Vec<Reason> {
    let mut reasons = Vec::new();
    let mut explained = BTreeSet::new();
    let mut pending = VecDeque::new();
    for &(sequence, rule) in roots {
        if retained.contains(&sequence) && explained.insert(sequence) {
            reasons.push(Reason {
                sequence,
                rule: rule.into(),
                via: None,
            });
            if reasons.len() == MAX_REASONS {
                return reasons;
            }
            pending.push_back(sequence);
        }
    }
    let mut outgoing: BTreeMap<u64, Vec<(u64, &'static str)>> = BTreeMap::new();
    for &(from, to, rule) in dependencies {
        outgoing.entry(from).or_default().push((to, rule));
    }
    let mut outgoing_groups: BTreeMap<u64, Vec<(usize, &'static str)>> = BTreeMap::new();
    for &(from, group, rule) in &groups.uses {
        outgoing_groups.entry(from).or_default().push((group, rule));
    }
    let mut members: BTreeMap<usize, Vec<u64>> = BTreeMap::new();
    for &(group, sequence) in &groups.members {
        members.entry(group).or_default().push(sequence);
    }
    let mut activated_groups = BTreeSet::new();
    while let Some(from) = pending.pop_front() {
        let mut edges = outgoing.get(&from).cloned().unwrap_or_default();
        if let Some(uses) = outgoing_groups.get(&from) {
            for &(group, rule) in uses {
                if activated_groups.insert(group)
                    && let Some(sequences) = members.get(&group)
                {
                    edges.extend(sequences.iter().map(|sequence| (*sequence, rule)));
                }
            }
        }
        edges.sort_unstable();
        edges.dedup();
        for (sequence, rule) in edges {
            if retained.contains(&sequence) && explained.insert(sequence) {
                reasons.push(Reason {
                    sequence,
                    rule: rule.into(),
                    via: Some(from),
                });
                if reasons.len() == MAX_REASONS {
                    return reasons;
                }
                pending.push_back(sequence);
            }
        }
    }
    reasons
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_collapses_duplicate_facts_and_requires_a_root_for_cycles() {
        let mut runtime = Crepe::new();
        runtime.extend([Root(1, "root"), Root(1, "root")]);
        runtime.extend([
            Dependency(1, 2, "edge"),
            Dependency(1, 2, "edge"),
            Dependency(2, 1, "rooted_cycle"),
            Dependency(3, 4, "isolated_cycle"),
            Dependency(4, 3, "isolated_cycle"),
        ]);
        runtime.extend([
            UsesGroup(2, 0, "group"),
            UsesGroup(2, 0, "group"),
            UsesGroup(5, 0, "group"),
            UsesGroup(3, 1, "isolated_group"),
        ]);
        runtime.extend([Member(0, 5), Member(0, 5), Member(0, 1), Member(1, 4)]);
        let (keep,) = runtime.run();
        let sequences: BTreeSet<_> = keep.into_iter().map(|Keep(sequence)| sequence).collect();
        assert_eq!(sequences, BTreeSet::from([1, 2, 5]));
    }

    #[test]
    fn dense_groups_need_a_root_and_expand_once_without_pairwise_facts() {
        let groups = Groups {
            uses: (0..9_000)
                .map(|sequence| (sequence, 0, "same_execution"))
                .collect(),
            members: (0..9_000).map(|sequence| (0, sequence)).collect(),
        };
        assert_eq!(groups.uses.len() + groups.members.len(), 18_000);
        let unrooted = evaluate([], [], groups.clone());
        assert!(unrooted.sequences.is_empty());
        let rooted = evaluate([(8_999, "root")], [], groups);
        assert_eq!(rooted.sequences.len(), 9_000);
        assert_eq!(rooted.reasons.len(), MAX_REASONS);
        assert!(
            rooted
                .reasons
                .iter()
                .skip(1)
                .all(|reason| reason.via == Some(8_999))
        );
    }

    #[test]
    fn grouped_closure_and_witnesses_match_expanded_dependencies() {
        for member_bits in 0_u8..8 {
            for uses_bits in 0_u8..8 {
                let groups = Groups {
                    uses: (0_u64..3)
                        .filter(|sequence| uses_bits & (1 << sequence) != 0)
                        .map(|sequence| (sequence, 0, "group"))
                        .collect(),
                    members: (0_u64..3)
                        .filter(|sequence| member_bits & (1 << sequence) != 0)
                        .map(|sequence| (0, sequence))
                        .collect(),
                };
                let explicit = [(2, 3, "explicit"), (3, 0, "cycle")];
                let expanded: Vec<_> = explicit
                    .into_iter()
                    .chain(groups.uses.iter().flat_map(|&(from, _, rule)| {
                        groups.members.iter().map(move |&(_, to)| (from, to, rule))
                    }))
                    .collect();
                for root_bits in 0_u8..8 {
                    let roots: Vec<_> = (0_u64..3)
                        .filter(|sequence| root_bits & (1 << sequence) != 0)
                        .map(|sequence| (sequence, "root"))
                        .collect();
                    let grouped = evaluate(roots.clone(), explicit, groups.clone());
                    let flat = evaluate(roots, expanded.clone(), Groups::default());
                    assert_eq!(grouped, flat);
                }
            }
        }
    }

    #[test]
    fn facts_and_explanations_ignore_order_duplicates_and_hash_iteration() {
        let roots = vec![(4, "z_root"), (9, "independent"), (4, "a_root")];
        let dependencies = vec![
            (4, 2, "z_edge"),
            (4, 2, "a_edge"),
            (2, 3, "next"),
            (3, 2, "cycle"),
            (2, 2, "self"),
            (7, 8, "isolated"),
            (8, 7, "isolated"),
        ];
        let expected = evaluate(roots.clone(), dependencies.clone(), Groups::default());
        assert_eq!(expected.sequences, BTreeSet::from([2, 3, 4, 9]));
        assert_eq!(
            expected.reasons,
            vec![
                Reason {
                    sequence: 4,
                    rule: "a_root".into(),
                    via: None
                },
                Reason {
                    sequence: 9,
                    rule: "independent".into(),
                    via: None
                },
                Reason {
                    sequence: 2,
                    rule: "a_edge".into(),
                    via: Some(4)
                },
                Reason {
                    sequence: 3,
                    rule: "next".into(),
                    via: Some(2)
                },
            ]
        );
        for rotation in 0..dependencies.len() {
            let mut shuffled_roots = roots.clone();
            shuffled_roots.reverse();
            shuffled_roots.extend(roots.clone());
            let mut shuffled_dependencies = dependencies.clone();
            shuffled_dependencies.rotate_left(rotation);
            shuffled_dependencies.extend(dependencies.iter().rev().copied());
            assert_eq!(
                evaluate(shuffled_roots, shuffled_dependencies, Groups::default()),
                expected
            );
        }
    }

    #[test]
    fn explanation_limit_does_not_limit_retention() {
        let dependencies = (0..299).map(|sequence| (sequence, sequence + 1, "next"));
        let result = evaluate([(0, "root")], dependencies, Groups::default());
        assert_eq!(result.sequences.len(), 300);
        assert_eq!(result.reasons.len(), MAX_REASONS);
        let mut witnessed = BTreeSet::new();
        for reason in result.reasons {
            if let Some(via) = reason.via {
                assert!(witnessed.contains(&via));
            }
            assert!(witnessed.insert(reason.sequence));
        }
    }

    #[test]
    fn all_three_node_graphs_match_independent_reachability_oracle() {
        // Includes every self-loop, isolated cycle, rooted cycle, diamond, and
        // choice of zero or multiple roots, independent of domain extraction.
        for edge_bits in 0_u16..512 {
            let mut edges = Vec::new();
            for from in 0_u64..3 {
                for to in 0_u64..3 {
                    if edge_bits & (1 << (from * 3 + to)) != 0 {
                        edges.push((from, to, "dependency"));
                    }
                }
            }
            for root_bits in 0_u8..8 {
                let roots: Vec<_> = (0_u64..3)
                    .filter(|sequence| root_bits & (1 << sequence) != 0)
                    .map(|sequence| (sequence, "root"))
                    .collect();
                let mut oracle: BTreeSet<_> = roots.iter().map(|&(seq, _)| seq).collect();
                let mut pending: VecDeque<_> = oracle.iter().copied().collect();
                while let Some(sequence) = pending.pop_front() {
                    for &(from, to, _) in &edges {
                        if from == sequence && oracle.insert(to) {
                            pending.push_back(to);
                        }
                    }
                }
                let result = evaluate(roots, edges.clone(), Groups::default());
                assert_eq!(
                    result.sequences, oracle,
                    "edges={edge_bits}, roots={root_bits}"
                );
                assert_eq!(result.reasons.len(), result.sequences.len());
                let mut witnessed = BTreeSet::new();
                for reason in result.reasons {
                    if let Some(via) = reason.via {
                        assert!(witnessed.contains(&via));
                    }
                    assert!(witnessed.insert(reason.sequence));
                }
            }
        }
    }
}
