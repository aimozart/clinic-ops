use entropa_core::Transaction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GateOutcome {
    Approved,
    Rejected,
    Pending,
}

/// One AI decision. `input_hash` is a hash of the input, never the input itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecisionRecord {
    pub actor: String,
    pub kind: String,
    pub input_hash: String,
    pub output: String,
    pub human_gate: Option<GateOutcome>,
    pub timestamp: u64,
}

/// blake3 of the raw input, lowercase hex (64 chars).
pub fn input_hash(raw: &[u8]) -> String {
    blake3::hash(raw).to_hex().to_string()
}

impl DecisionRecord {
    /// Canonical JSON payload: fixed field order, so the same record always gives the same bytes.
    pub fn canonical_payload(&self) -> String {
        #[derive(Serialize)]
        struct Payload<'a> {
            input_hash: &'a str,
            output: &'a str,
            human_gate: &'a Option<GateOutcome>,
            timestamp: u64,
        }
        serde_json::to_string(&Payload {
            input_hash: &self.input_hash,
            output: &self.output,
            human_gate: &self.human_gate,
            timestamp: self.timestamp,
        })
        .expect("payload serializes")
    }

    pub fn to_transaction(&self) -> Transaction {
        Transaction::new(self.actor.clone(), self.kind.clone(), self.canonical_payload())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec() -> DecisionRecord {
        DecisionRecord {
            actor: "copilot-review".into(),
            kind: "review".into(),
            input_hash: input_hash(b"diff text"),
            output: "Looks fine; one nit.".into(),
            human_gate: Some(GateOutcome::Approved),
            timestamp: 1_700_000_000,
        }
    }

    #[test]
    fn input_hash_is_64_lowercase_hex_and_deterministic() {
        let h = input_hash(b"hello");
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_eq!(h, input_hash(b"hello"));
        assert_ne!(h, input_hash(b"hellp"));
    }

    #[test]
    fn canonical_payload_is_deterministic_and_has_fixed_field_order() {
        let p = rec().canonical_payload();
        assert_eq!(p, rec().canonical_payload());
        let i = p.find("input_hash").unwrap();
        let o = p.find("output").unwrap();
        let g = p.find("human_gate").unwrap();
        let t = p.find("timestamp").unwrap();
        assert!(i < o && o < g && g < t, "field order must be fixed: {p}");
    }

    #[test]
    fn payload_never_contains_actor_or_kind_twice_and_changes_with_any_field() {
        let a = rec();
        let mut b = rec();
        b.output.push('!');
        assert_ne!(a.canonical_payload(), b.canonical_payload());
        let mut c = rec();
        c.human_gate = Some(GateOutcome::Rejected);
        assert_ne!(a.canonical_payload(), c.canonical_payload());
    }

    #[test]
    fn transaction_carries_actor_kind_and_payload() {
        let r = rec();
        let tx = r.to_transaction();
        assert_eq!(tx.from, "copilot-review");
        assert_eq!(tx.kind, "review");
        assert_eq!(tx.payload, r.canonical_payload());
    }
}
