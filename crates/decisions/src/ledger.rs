use crate::checker::{check, Violation};
use crate::record::DecisionRecord;
use entropa_core::{verify_hex, Chain, ChainError, Probe};

const BEACON: &str = "clinic-ops-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub index: u64,
    pub content_hash: String,
    pub block_hash: String,
    pub signature: String,
    pub pubkey: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error("record rejected by the checker: {0:?}")]
    Rejected(Vec<Violation>),
    #[error("chain error: {0:?}")]
    Chain(ChainError),
}

pub struct Ledger {
    chain: Chain,
    probe: Probe,
}

impl Ledger {
    pub fn new(probe: Probe, timestamp: u64) -> Self {
        let chain = Chain::genesis(&probe, timestamp, BEACON);
        Self { chain, probe }
    }

    /// Check the record, then sign and append it. A record that fails the checker is refused
    /// and the chain is left untouched.
    pub fn record(&mut self, r: &DecisionRecord, timestamp: u64) -> Result<Receipt, LedgerError> {
        let violations = check(r);
        if !violations.is_empty() {
            return Err(LedgerError::Rejected(violations));
        }
        let tx = r.to_transaction();
        let content_hash = tx.content_hash();
        let block = self.chain.draft(&self.probe, timestamp, BEACON, vec![tx]);
        let receipt = Receipt {
            index: block.index,
            content_hash,
            block_hash: block.hash.clone(),
            signature: block.signature.clone(),
            pubkey: block.proposer_pubkey.clone(),
        };
        self.chain.try_append(block).map_err(LedgerError::Chain)?;
        Ok(receipt)
    }

    pub fn verify(&self) -> Result<(), ChainError> {
        self.chain.verify()
    }

    /// True only if this ledger holds that exact block, with that content, signed by that key.
    pub fn verify_receipt(&self, r: &Receipt) -> bool {
        let Some(b) = self.chain.blocks.iter().find(|b| b.index == r.index) else {
            return false;
        };
        let Ok(digest) = hex::decode(&r.block_hash) else {
            return false;
        };
        b.hash == r.block_hash
            && b.signature == r.signature
            && b.proposer_pubkey == r.pubkey
            && b.transactions.iter().any(|t| t.content_hash() == r.content_hash)
            && verify_hex(&r.pubkey, &digest, &r.signature)
    }

    pub fn len(&self) -> usize {
        self.chain.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn head_hash(&self) -> String {
        self.chain.head().map(|b| b.hash.clone()).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{input_hash, GateOutcome};

    fn rec(n: u8) -> DecisionRecord {
        DecisionRecord {
            actor: "copilot-review".into(),
            kind: "review".into(),
            input_hash: input_hash(&[n]),
            output: format!("decision {n}"),
            human_gate: Some(GateOutcome::Approved),
            timestamp: 1_700_000_000 + n as u64,
        }
    }

    fn ledger() -> Ledger {
        Ledger::new(Probe::spawn(), 1_700_000_000)
    }

    #[test]
    fn new_ledger_has_only_genesis_and_verifies() {
        let l = ledger();
        assert_eq!(l.len(), 1);
        assert!(l.verify().is_ok());
    }

    #[test]
    fn recording_returns_a_receipt_and_chain_still_verifies() {
        let mut l = ledger();
        let r = l.record(&rec(1), 1_700_000_001).unwrap();
        assert_eq!(r.index, 1);
        assert_eq!(l.len(), 2);
        assert_eq!(r.block_hash, l.head_hash());
        assert!(l.verify().is_ok());
        assert!(l.verify_receipt(&r));
    }

    #[test]
    fn many_records_chain_and_every_receipt_verifies() {
        let mut l = ledger();
        let receipts: Vec<_> = (1..=5).map(|n| l.record(&rec(n), 1_700_000_000 + n as u64).unwrap()).collect();
        assert_eq!(l.len(), 6);
        assert!(l.verify().is_ok());
        assert!(receipts.iter().all(|r| l.verify_receipt(r)));
        let idx: Vec<_> = receipts.iter().map(|r| r.index).collect();
        assert_eq!(idx, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn a_record_that_fails_the_checker_is_refused_and_leaves_the_chain_unchanged() {
        let mut l = ledger();
        let mut bad = rec(1);
        bad.output = "SSN 123-45-6789".into();
        let before = l.head_hash();
        match l.record(&bad, 1_700_000_001) {
            Err(LedgerError::Rejected(v)) => assert_eq!(v, vec![Violation::PhiPattern("ssn")]),
            other => panic!("expected Rejected, got {other:?}"),
        }
        assert_eq!(l.len(), 1);
        assert_eq!(l.head_hash(), before);
    }

    #[test]
    fn tampering_with_a_recorded_decision_breaks_verification() {
        let mut l = ledger();
        l.record(&rec(1), 1_700_000_001).unwrap();
        l.record(&rec(2), 1_700_000_002).unwrap();
        l.chain.blocks[1].transactions[0].payload.push('X');
        assert!(l.verify().is_err());
    }

    #[test]
    fn a_forged_receipt_does_not_verify() {
        let mut l = ledger();
        let mut r = l.record(&rec(1), 1_700_000_001).unwrap();
        r.signature.replace_range(0..2, if r.signature.starts_with("00") { "ff" } else { "00" });
        assert!(!l.verify_receipt(&r));
        let mut r2 = l.record(&rec(2), 1_700_000_002).unwrap();
        r2.block_hash = "0".repeat(64);
        assert!(!l.verify_receipt(&r2));
        let r3 = Receipt { index: 99, ..r2.clone() };
        assert!(!l.verify_receipt(&r3));
    }

    #[test]
    fn a_receipt_signed_by_a_different_key_does_not_verify() {
        let mut a = ledger();
        let mut b = ledger();
        let ra = a.record(&rec(1), 1_700_000_001).unwrap();
        b.record(&rec(1), 1_700_000_001).unwrap();
        assert!(!b.verify_receipt(&ra));
    }
}
