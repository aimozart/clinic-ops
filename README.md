# clinic-ops

Rust workspace for the Clinic Ops capstone: AI decision records, a deterministic checker, and a post-quantum signed ledger built on the published `entropa-core` crate (ML-DSA-65, NIST FIPS 204). Synthetic data only. Built with AI assistance, test first.

## Run the tests (Rust runs in a container on this PC)
    ./run-cargo.sh test

## What a signature proves
Who recorded what, and that it was not changed afterwards. It does not prove an AI decision was correct. This is audit-control engineering, not a compliance claim.

## Crates
- `decisions`: `DecisionRecord`, `checker::check` (allowed kinds, input must be a hash, output length, PHI-like patterns, human gate present), `Ledger` (check, sign, chain, receipts, verify).
- `clinic-api`: axum API. `GET /health`, `/appointments`, `/summary` (synthetic, seeded), `POST /decisions` (raw input is hashed and dropped; checker runs; ML-DSA signed receipt), `GET /decisions`, `POST /verify`, `GET /chain/head`. CORS limited to one origin (`CORS_ORIGIN`). Ledger and key are in memory for now (restart = new chain; key custody via Key Vault is a later step).
- Next: `clinic-mcp` (read-only MCP server over stdio), Entropa anchoring client, Dockerfile + CI.
