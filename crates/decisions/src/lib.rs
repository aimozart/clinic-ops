//! Clinic Ops decision audit layer.
//!
//! Every AI decision becomes a [`DecisionRecord`]; a deterministic [`checker`] must pass it;
//! a [`ledger::Ledger`] then signs it (ML-DSA-65, via `entropa-core`) and chains it.
//! A signature proves WHO recorded WHAT and that it was not changed. It does not prove the
//! AI decision was correct.

pub mod checker;
pub mod ledger;
pub mod record;

pub use checker::{check, Violation, ALLOWED_KINDS, MAX_OUTPUT};
pub use ledger::{Ledger, LedgerError, Receipt};
pub use record::{input_hash, DecisionRecord, GateOutcome};
