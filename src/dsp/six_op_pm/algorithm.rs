#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_graphs_prepare_and_have_unique_connectivity() {
        assert_eq!(CLASSIC_ALGORITHMS.len(), 32);
        let mut fingerprints = Vec::new();
        for spec in CLASSIC_ALGORITHMS {
            let prepared = PreparedAlgorithm::new(spec).unwrap();
            assert_ne!(prepared.carrier_mask(), 0);
            for edge in spec.edges {
                assert!(
                    prepared.position(edge.source + 1).unwrap()
                        < prepared.position(edge.target + 1).unwrap()
                );
            }
            fingerprints.push(spec.fingerprint());
        }
        fingerprints.sort_unstable();
        fingerprints.dedup();
        assert_eq!(fingerprints.len(), 32);
    }

    #[test]
    fn group_feedback_is_removed_before_topological_sort() {
        let prepared = PreparedAlgorithm::new(CLASSIC_ALGORITHMS[3]).unwrap();
        assert_eq!(prepared.feedback(), Edge::one_based(4, 6));
        assert!(prepared.position(6).unwrap() < prepared.position(5).unwrap());
        assert!(prepared.position(5).unwrap() < prepared.position(4).unwrap());
    }

    #[test]
    fn undeclared_cycle_and_carrierless_graph_are_rejected() {
        const CYCLE: AlgorithmSpec = AlgorithmSpec::new(
            99,
            &[Edge::one_based(1, 2), Edge::one_based(2, 1)],
            carrier_mask(&[1]),
            Edge::one_based(6, 6),
        );
        const NO_CARRIER: AlgorithmSpec = AlgorithmSpec::new(100, &[], 0, Edge::one_based(6, 6));
        assert_eq!(PreparedAlgorithm::new(CYCLE), Err(AlgorithmError::Cycle));
        assert_eq!(
            PreparedAlgorithm::new(NO_CARRIER),
            Err(AlgorithmError::Carrierless)
        );
    }

    #[test]
    fn duplicate_ordinary_edges_are_rejected() {
        const DUPLICATE: AlgorithmSpec = AlgorithmSpec::new(
            101,
            &[Edge::one_based(2, 1), Edge::one_based(2, 1)],
            carrier_mask(&[1]),
            Edge::one_based(6, 6),
        );

        assert_eq!(
            PreparedAlgorithm::new(DUPLICATE),
            Err(AlgorithmError::DuplicateEdge)
        );
    }

    #[test]
    fn ordinary_edge_cannot_repeat_declared_feedback_edge() {
        const FEEDBACK_OVERLAP: AlgorithmSpec = AlgorithmSpec::new(
            102,
            &[Edge::one_based(2, 1)],
            carrier_mask(&[1]),
            Edge::one_based(2, 1),
        );

        assert_eq!(
            PreparedAlgorithm::new(FEEDBACK_OVERLAP),
            Err(AlgorithmError::DuplicateEdge)
        );
    }

    #[test]
    fn one_based_edges_preserve_invalid_indices_for_validation() {
        const ZERO_SOURCE: AlgorithmSpec = AlgorithmSpec::new(
            103,
            &[Edge::one_based(0, 1)],
            carrier_mask(&[1]),
            Edge::one_based(6, 6),
        );
        const SEVEN_SOURCE: AlgorithmSpec = AlgorithmSpec::new(
            104,
            &[Edge::one_based(7, 1)],
            carrier_mask(&[1]),
            Edge::one_based(6, 6),
        );
        const ZERO_TARGET: AlgorithmSpec = AlgorithmSpec::new(
            105,
            &[Edge::one_based(1, 0)],
            carrier_mask(&[1]),
            Edge::one_based(6, 6),
        );
        const SEVEN_TARGET: AlgorithmSpec = AlgorithmSpec::new(
            106,
            &[Edge::one_based(1, 7)],
            carrier_mask(&[1]),
            Edge::one_based(6, 6),
        );

        for spec in [ZERO_SOURCE, SEVEN_SOURCE, ZERO_TARGET, SEVEN_TARGET] {
            assert_eq!(
                PreparedAlgorithm::new(spec),
                Err(AlgorithmError::InvalidOperator)
            );
        }
    }

    #[test]
    fn carrier_masks_preserve_invalid_operators_for_validation() {
        const ZERO_CARRIER: AlgorithmSpec =
            AlgorithmSpec::new(107, &[], carrier_mask(&[0]), Edge::one_based(6, 6));
        const SEVEN_CARRIER: AlgorithmSpec =
            AlgorithmSpec::new(108, &[], carrier_mask(&[7]), Edge::one_based(6, 6));
        const INVALID_MASK: AlgorithmSpec =
            AlgorithmSpec::new(109, &[], u8::MAX, Edge::one_based(6, 6));

        for spec in [ZERO_CARRIER, SEVEN_CARRIER, INVALID_MASK] {
            assert_eq!(
                PreparedAlgorithm::new(spec),
                Err(AlgorithmError::InvalidOperator)
            );
        }
    }

    #[test]
    fn accessors_are_checked_and_return_exact_incoming_sources() {
        let prepared = PreparedAlgorithm::new(CLASSIC_ALGORITHMS[11]).unwrap();

        assert_eq!(prepared.incoming(3).unwrap(), &[3, 4, 5]);
        assert_eq!(prepared.is_carrier(1), Some(true));
        assert_eq!(prepared.is_carrier(2), Some(false));
        assert_eq!(prepared.position(0), None);
        assert_eq!(prepared.position(7), None);
        assert_eq!(prepared.incoming(0), None);
        assert_eq!(prepared.incoming(7), None);
        assert_eq!(prepared.is_carrier(0), None);
        assert_eq!(prepared.is_carrier(7), None);
    }

    #[test]
    fn fingerprints_ignore_ordinary_edge_order() {
        const FORWARD: AlgorithmSpec = AlgorithmSpec::new(
            110,
            &[Edge::one_based(3, 2), Edge::one_based(2, 1)],
            carrier_mask(&[1]),
            Edge::one_based(6, 6),
        );
        const REVERSED: AlgorithmSpec = AlgorithmSpec::new(
            111,
            &[Edge::one_based(2, 1), Edge::one_based(3, 2)],
            carrier_mask(&[1]),
            Edge::one_based(6, 6),
        );

        assert_eq!(FORWARD.fingerprint(), REVERSED.fingerprint());
    }

    #[test]
    fn fingerprint_is_safe_and_canonical_for_malformed_edges() {
        const FIRST: AlgorithmSpec = AlgorithmSpec::new(
            112,
            &[
                Edge {
                    source: u8::MAX,
                    target: 0,
                },
                Edge {
                    source: 0,
                    target: u8::MAX,
                },
            ],
            u8::MAX,
            Edge {
                source: u8::MAX,
                target: u8::MAX,
            },
        );
        const REVERSED: AlgorithmSpec = AlgorithmSpec::new(
            113,
            &[
                Edge {
                    source: 0,
                    target: u8::MAX,
                },
                Edge {
                    source: u8::MAX,
                    target: 0,
                },
            ],
            u8::MAX,
            Edge {
                source: u8::MAX,
                target: u8::MAX,
            },
        );

        assert_eq!(FIRST.fingerprint(), REVERSED.fingerprint());
    }
}
use thiserror::Error;

pub const OPERATOR_COUNT: usize = 6;
const VALID_CARRIER_BITS: u8 = (1 << OPERATOR_COUNT) - 1;
const INVALID_CARRIER_BIT: u8 = 1 << OPERATOR_COUNT;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Edge {
    pub source: u8,
    pub target: u8,
}

impl Edge {
    pub const fn one_based(source: u8, target: u8) -> Self {
        Self {
            source: source.wrapping_sub(1),
            target: target.wrapping_sub(1),
        }
    }
}

pub const fn carrier_mask(operators: &[u8]) -> u8 {
    let mut mask = 0;
    let mut index = 0;
    while index < operators.len() {
        let operator = operators[index];
        if operator > 0 && operator <= OPERATOR_COUNT as u8 {
            mask |= 1 << (operator - 1);
        } else {
            mask |= INVALID_CARRIER_BIT;
        }
        index += 1;
    }
    mask
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AlgorithmSpec {
    pub id: u8,
    pub edges: &'static [Edge],
    pub carriers: u8,
    pub feedback: Edge,
}

impl AlgorithmSpec {
    pub const fn new(id: u8, edges: &'static [Edge], carriers: u8, feedback: Edge) -> Self {
        Self {
            id,
            edges,
            carriers,
            feedback,
        }
    }

    pub fn fingerprint(self) -> u64 {
        let mut adjacency = 0_u64;
        let mut malformed_edge = false;
        for edge in self.edges {
            if usize::from(edge.source) < OPERATOR_COUNT
                && usize::from(edge.target) < OPERATOR_COUNT
            {
                let bit = usize::from(edge.source) * OPERATOR_COUNT + usize::from(edge.target);
                adjacency |= 1_u64 << bit;
            } else {
                malformed_edge = true;
            }
        }

        adjacency
            | (u64::from(self.carriers) << 36)
            | (u64::from(self.feedback.source) << 44)
            | (u64::from(self.feedback.target) << 52)
            | (u64::from(malformed_edge) << 60)
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum AlgorithmError {
    #[error("invalid operator index")]
    InvalidOperator,
    #[error("duplicate edge")]
    DuplicateEdge,
    #[error("no carrier")]
    Carrierless,
    #[error("undeclared cycle")]
    Cycle,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreparedAlgorithm {
    incoming: [[u8; OPERATOR_COUNT]; OPERATOR_COUNT],
    incoming_count: [u8; OPERATOR_COUNT],
    order: [u8; OPERATOR_COUNT],
    carriers: u8,
    feedback: Edge,
    output_gain: f32,
}

impl PreparedAlgorithm {
    pub fn new(spec: AlgorithmSpec) -> Result<Self, AlgorithmError> {
        validate_edge(spec.feedback)?;
        if spec.carriers == 0 {
            return Err(AlgorithmError::Carrierless);
        }
        if spec.carriers & !VALID_CARRIER_BITS != 0 {
            return Err(AlgorithmError::InvalidOperator);
        }

        let mut incoming = [[0; OPERATOR_COUNT]; OPERATOR_COUNT];
        let mut incoming_count = [0; OPERATOR_COUNT];
        let mut indegree = [0; OPERATOR_COUNT];

        for (index, edge) in spec.edges.iter().copied().enumerate() {
            validate_edge(edge)?;
            if edge == spec.feedback || spec.edges[..index].contains(&edge) {
                return Err(AlgorithmError::DuplicateEdge);
            }

            let target = usize::from(edge.target);
            let count = usize::from(incoming_count[target]);
            incoming[target][count] = edge.source;
            incoming_count[target] += 1;
            indegree[target] += 1;
        }

        let mut order = [0; OPERATOR_COUNT];
        let mut selected = [false; OPERATOR_COUNT];
        for order_slot in &mut order {
            let mut next = None;
            for (operator, was_selected) in selected.iter().enumerate() {
                if !was_selected && indegree[operator] == 0 {
                    next = Some(operator);
                    break;
                }
            }
            let Some(operator) = next else {
                return Err(AlgorithmError::Cycle);
            };

            selected[operator] = true;
            *order_slot = operator as u8;
            for edge in spec.edges {
                if usize::from(edge.source) == operator {
                    indegree[usize::from(edge.target)] -= 1;
                }
            }
        }

        let carrier_count = spec.carriers.count_ones() as f32;
        Ok(Self {
            incoming,
            incoming_count,
            order,
            carriers: spec.carriers,
            feedback: spec.feedback,
            output_gain: 0.75 / carrier_count.sqrt(),
        })
    }

    pub const fn carrier_mask(&self) -> u8 {
        self.carriers
    }

    pub const fn feedback(&self) -> Edge {
        self.feedback
    }

    pub fn position(&self, operator: u8) -> Option<usize> {
        let operator = u8::try_from(one_based_index(operator)?).ok()?;
        let mut position = 0;
        while position < OPERATOR_COUNT {
            if self.order[position] == operator {
                return Some(position);
            }
            position += 1;
        }
        None
    }

    pub const fn evaluation_order(&self) -> &[u8; OPERATOR_COUNT] {
        &self.order
    }

    pub fn incoming(&self, operator: u8) -> Option<&[u8]> {
        let operator = one_based_index(operator)?;
        Some(&self.incoming[operator][..usize::from(self.incoming_count[operator])])
    }

    pub const fn is_carrier(&self, operator: u8) -> Option<bool> {
        if operator == 0 || operator > OPERATOR_COUNT as u8 {
            return None;
        }
        Some(self.carriers & (1 << (operator - 1)) != 0)
    }

    pub const fn output_gain(&self) -> f32 {
        self.output_gain
    }
}

fn one_based_index(operator: u8) -> Option<usize> {
    let index = usize::from(operator.checked_sub(1)?);
    (index < OPERATOR_COUNT).then_some(index)
}

fn validate_edge(edge: Edge) -> Result<(), AlgorithmError> {
    if usize::from(edge.source) >= OPERATOR_COUNT || usize::from(edge.target) >= OPERATOR_COUNT {
        return Err(AlgorithmError::InvalidOperator);
    }
    Ok(())
}

const fn a(id: u8, edges: &'static [Edge], carriers: &[u8], feedback: Edge) -> AlgorithmSpec {
    AlgorithmSpec::new(id, edges, carrier_mask(carriers), feedback)
}

const fn e(source: u8, target: u8) -> Edge {
    Edge::one_based(source, target)
}

// Connectivity facts transcribed from the official Yamaha PLG100-DX Owner's
// Manual algorithm chart, pages 28-29:
// https://usa.yamaha.com/files/download/other_assets/1/320951/PLG100DXE.pdf
// The delayed feedback edge is intentionally excluded from each ordinary edge list.
pub const CLASSIC_ALGORITHMS: [AlgorithmSpec; 32] = [
    a(1, &[e(6, 5), e(5, 4), e(4, 3), e(2, 1)], &[1, 3], e(6, 6)),
    a(2, &[e(6, 5), e(5, 4), e(4, 3), e(2, 1)], &[1, 3], e(2, 2)),
    a(3, &[e(3, 2), e(2, 1), e(6, 5), e(5, 4)], &[1, 4], e(6, 6)),
    a(4, &[e(3, 2), e(2, 1), e(6, 5), e(5, 4)], &[1, 4], e(4, 6)),
    a(5, &[e(2, 1), e(4, 3), e(6, 5)], &[1, 3, 5], e(6, 6)),
    a(6, &[e(2, 1), e(4, 3), e(6, 5)], &[1, 3, 5], e(5, 6)),
    a(7, &[e(2, 1), e(4, 3), e(6, 5), e(5, 3)], &[1, 3], e(6, 6)),
    a(8, &[e(2, 1), e(4, 3), e(6, 5), e(5, 3)], &[1, 3], e(4, 4)),
    a(9, &[e(2, 1), e(4, 3), e(6, 5), e(5, 3)], &[1, 3], e(2, 2)),
    a(
        10,
        &[e(3, 2), e(2, 1), e(5, 4), e(6, 4), e(4, 1)],
        &[1],
        e(3, 3),
    ),
    a(
        11,
        &[e(3, 2), e(2, 1), e(5, 4), e(6, 4), e(4, 1)],
        &[1],
        e(6, 6),
    ),
    a(
        12,
        &[e(2, 1), e(4, 3), e(5, 3), e(6, 3), e(3, 1)],
        &[1],
        e(2, 2),
    ),
    a(
        13,
        &[e(4, 3), e(5, 3), e(6, 3), e(3, 1), e(2, 1)],
        &[1],
        e(6, 6),
    ),
    a(14, &[e(5, 2), e(2, 1), e(6, 4), e(4, 3)], &[1, 3], e(6, 6)),
    a(15, &[e(5, 2), e(2, 1), e(6, 4), e(4, 3)], &[1, 3], e(2, 2)),
    a(
        16,
        &[e(2, 1), e(4, 3), e(3, 1), e(6, 5), e(5, 1)],
        &[1],
        e(6, 6),
    ),
    a(
        17,
        &[e(2, 1), e(4, 3), e(3, 1), e(6, 5), e(5, 1)],
        &[1],
        e(2, 2),
    ),
    a(
        18,
        &[e(2, 1), e(3, 1), e(6, 5), e(5, 4), e(4, 1)],
        &[1],
        e(3, 3),
    ),
    a(
        19,
        &[e(3, 2), e(2, 1), e(6, 4), e(6, 5)],
        &[1, 4, 5],
        e(6, 6),
    ),
    a(20, &[e(3, 1), e(5, 2), e(6, 4)], &[1, 2, 4], e(3, 3)),
    a(21, &[e(3, 1), e(6, 4)], &[1, 2, 4, 5], e(3, 3)),
    a(
        22,
        &[e(2, 1), e(6, 3), e(6, 4), e(6, 5)],
        &[1, 3, 4, 5],
        e(6, 6),
    ),
    a(
        23,
        &[e(3, 1), e(6, 2), e(6, 4), e(6, 5)],
        &[1, 2, 4, 5],
        e(6, 6),
    ),
    a(
        24,
        &[e(6, 1), e(6, 2), e(6, 3), e(6, 4), e(6, 5)],
        &[1, 2, 3, 4, 5],
        e(6, 6),
    ),
    a(25, &[e(6, 4)], &[1, 2, 3, 4, 5], e(6, 6)),
    a(26, &[e(3, 2), e(5, 4), e(6, 4)], &[1, 2, 4], e(6, 6)),
    a(27, &[e(3, 2), e(5, 4), e(6, 4)], &[1, 2, 4], e(3, 3)),
    a(28, &[e(2, 1), e(5, 4), e(4, 3)], &[1, 3, 6], e(5, 5)),
    a(29, &[e(4, 3), e(6, 5)], &[1, 2, 3, 5], e(6, 6)),
    a(30, &[e(5, 4), e(4, 3)], &[1, 2, 3, 6], e(5, 5)),
    a(31, &[e(6, 5)], &[1, 2, 3, 4, 5], e(6, 6)),
    a(32, &[], &[1, 2, 3, 4, 5, 6], e(6, 6)),
];
