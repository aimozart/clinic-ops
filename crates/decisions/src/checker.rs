use crate::record::DecisionRecord;
use regex::Regex;
use std::sync::OnceLock;

pub const ALLOWED_KINDS: [&str; 3] = ["review", "triage", "kql_explain"];
pub const MAX_OUTPUT: usize = 2000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    UnknownKind(String),
    InputNotAHash,
    OutputTooLong,
    PhiPattern(&'static str),
    GateMissing,
}

/// Deterministic policy check. Returns every violation found (empty = passes).
pub fn check(r: &DecisionRecord) -> Vec<Violation> {
    let mut v = Vec::new();
    if !ALLOWED_KINDS.contains(&r.kind.as_str()) {
        v.push(Violation::UnknownKind(r.kind.clone()));
    }
    let is_hash = r.input_hash.len() == 64 && r.input_hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    if !is_hash {
        v.push(Violation::InputNotAHash);
    }
    if r.output.chars().count() > MAX_OUTPUT {
        v.push(Violation::OutputTooLong);
    }
    for (name, re) in phi_patterns() {
        if re.is_match(&r.output) {
            v.push(Violation::PhiPattern(name));
        }
    }
    if r.human_gate.is_none() {
        v.push(Violation::GateMissing);
    }
    v
}

fn phi_patterns() -> &'static [(&'static str, Regex)] {
    static P: OnceLock<Vec<(&'static str, Regex)>> = OnceLock::new();
    P.get_or_init(|| {
        vec![
            ("ssn", Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap()),
            ("email", Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").unwrap()),
            ("phone", Regex::new(r"(\(\d{3}\)\s?|\b\d{3}[-.\s])\d{3}[-.\s]\d{4}\b").unwrap()),
            ("mrn", Regex::new(r"(?i)\bMRN[:#\s]*\d{5,}\b").unwrap()),
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{input_hash, GateOutcome};

    fn ok() -> DecisionRecord {
        DecisionRecord {
            actor: "failure-triage".into(),
            kind: "triage".into(),
            input_hash: input_hash(b"log"),
            output: "Deploy failed: health probe timed out.".into(),
            human_gate: Some(GateOutcome::Pending),
            timestamp: 1,
        }
    }

    #[test]
    fn a_clean_record_passes() {
        assert_eq!(check(&ok()), vec![]);
    }

    #[test]
    fn unknown_kind_is_rejected() {
        let mut r = ok();
        r.kind = "deploy_to_prod".into();
        assert_eq!(check(&r), vec![Violation::UnknownKind("deploy_to_prod".into())]);
    }

    #[test]
    fn raw_input_instead_of_a_hash_is_rejected() {
        let mut r = ok();
        r.input_hash = "patient John Smith notes".into();
        assert!(check(&r).contains(&Violation::InputNotAHash));
        let mut r2 = ok();
        r2.input_hash = "A".repeat(64); // uppercase is not our canonical form
        assert!(check(&r2).contains(&Violation::InputNotAHash));
    }

    #[test]
    fn output_length_boundary() {
        let mut r = ok();
        r.output = "x".repeat(MAX_OUTPUT);
        assert_eq!(check(&r), vec![]);
        r.output = "x".repeat(MAX_OUTPUT + 1);
        assert_eq!(check(&r), vec![Violation::OutputTooLong]);
    }

    #[test]
    fn phi_like_patterns_are_rejected() {
        for (text, name) in [
            ("patient SSN 123-45-6789 here", "ssn"),
            ("contact jane.doe@example.com now", "email"),
            ("call (602) 555-0142 today", "phone"),
            ("call 602-555-0142 today", "phone"),
            ("see MRN 00123456 for details", "mrn"),
        ] {
            let mut r = ok();
            r.output = text.into();
            assert_eq!(check(&r), vec![Violation::PhiPattern(name)], "text: {text}");
        }
    }

    #[test]
    fn plain_numbers_and_dates_are_not_phi_false_positives() {
        let mut r = ok();
        r.output = "Run 12345 took 3.5s on 2026-10-17; 4 of 7 checks passed.".into();
        assert_eq!(check(&r), vec![]);
    }

    #[test]
    fn missing_human_gate_is_flagged() {
        let mut r = ok();
        r.human_gate = None;
        assert_eq!(check(&r), vec![Violation::GateMissing]);
    }

    #[test]
    fn a_rejected_decision_is_still_a_valid_record() {
        let mut r = ok();
        r.human_gate = Some(GateOutcome::Rejected);
        assert_eq!(check(&r), vec![]);
    }

    #[test]
    fn multiple_violations_are_all_reported() {
        let mut r = ok();
        r.kind = "x".into();
        r.human_gate = None;
        let v = check(&r);
        assert!(v.contains(&Violation::UnknownKind("x".into())) && v.contains(&Violation::GateMissing));
    }
}
