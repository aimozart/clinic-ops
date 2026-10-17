# clinic-ops

Rust workspace for the Clinic Ops capstone: AI decision records, a deterministic checker, and a post-quantum signed ledger built on the published `entropa-core` crate (ML-DSA-65, NIST FIPS 204). Synthetic data only. Built with AI assistance, test first.

## Run the tests (Rust runs in a container on this PC)
    ./run-cargo.sh test

## What a signature proves
Who recorded what, and that it was not changed afterwards. It does not prove an AI decision was correct. This is audit-control engineering, not a compliance claim.

## Crates
- `decisions`: `DecisionRecord`, `checker::check` (allowed kinds, input must be a hash, output length, PHI-like patterns, human gate present), `Ledger` (check, sign, chain, receipts, verify).
- Next: `clinic-api` (synthetic clinic API + append/verify endpoints), `clinic-mcp` (read-only MCP server over stdio).
