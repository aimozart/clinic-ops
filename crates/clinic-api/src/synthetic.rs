//! Deterministic synthetic clinic data. No real people, ever: names are "Synthetic Patient NNNN".
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Appointment {
    pub id: u32,
    pub patient: String,
    pub clinic: String,
    pub day: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Summary {
    pub total: usize,
    pub by_status: BTreeMap<String, usize>,
}

/// `n` appointments, fully determined by `seed`.
pub fn appointments(seed: u64, n: usize) -> Vec<Appointment> {
    const STATUSES: [&str; 4] = ["scheduled", "completed", "cancelled", "no_show"];
    const CLINICS: [&str; 3] = ["North Clinic", "South Clinic", "East Clinic"];
    let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let mut next = move || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (state >> 33) as usize
    };
    (0..n)
        .map(|i| Appointment {
            id: i as u32 + 1,
            patient: format!("Synthetic Patient {:04}", i + 1),
            clinic: CLINICS[next() % CLINICS.len()].to_string(),
            day: format!("2026-11-{:02}", next() % 28 + 1),
            status: STATUSES[next() % STATUSES.len()].to_string(),
        })
        .collect()
}

pub fn summary(a: &[Appointment]) -> Summary {
    let mut by_status = BTreeMap::new();
    for x in a {
        *by_status.entry(x.status.clone()).or_insert(0) += 1;
    }
    Summary { total: a.len(), by_status }
}

#[cfg(test)]
mod tests {
    use super::*;
    use decisions::{check, DecisionRecord, GateOutcome, Violation};

    #[test]
    fn same_seed_same_data_different_seed_different_data() {
        assert_eq!(appointments(7, 20), appointments(7, 20));
        assert_ne!(appointments(7, 20), appointments(8, 20));
    }

    #[test]
    fn ids_run_from_one_and_names_are_obviously_synthetic() {
        let a = appointments(1, 12);
        assert_eq!(a.len(), 12);
        assert_eq!(a.iter().map(|x| x.id).collect::<Vec<_>>(), (1..=12).collect::<Vec<_>>());
        assert!(a.iter().all(|x| x.patient.starts_with("Synthetic Patient ")));
    }

    #[test]
    fn summary_counts_add_up() {
        let a = appointments(3, 50);
        let s = summary(&a);
        assert_eq!(s.total, 50);
        assert_eq!(s.by_status.values().sum::<usize>(), 50);
    }

    #[test]
    fn generated_data_contains_no_phi_like_patterns() {
        let json = serde_json::to_string(&appointments(5, 200)).unwrap();
        let rec = DecisionRecord {
            actor: "t".into(),
            kind: "review".into(),
            input_hash: "0".repeat(64),
            output: json.chars().take(2000).collect(),
            human_gate: Some(GateOutcome::Approved),
            timestamp: 1,
        };
        let phi: Vec<_> = check(&rec).into_iter().filter(|v| matches!(v, Violation::PhiPattern(_))).collect();
        assert!(phi.is_empty(), "synthetic data must not look like PHI: {phi:?}");
    }
}
