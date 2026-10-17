# Testing
Tests are co-located (`#[cfg(test)] mod tests`), the idiomatic Rust layout. A scanner that only counts files under `tests/` will undercount (see the same note in entropa-public ANALYSIS_NOTES.md).

    ./run-cargo.sh test

Red/green record (2026-10-17): with stub functions, 0 passed and 20 failed ("not yet implemented"); after implementation, 20 passed, 0 failed.

## clinic-api (2026-10-17)
- Red: with `todo!()` handlers, 0 passed and 15 failed. Green after implementation: 15 passed. One fix along the way: tower-http's exact-origin CORS setting sends the allowed origin to every caller, so the CORS test failed; switched to an origin predicate so only the configured origin gets the header.
- `decisions` grew to 22 tests. Of the two added, `blocks_and_public_key_are_exposed` was seen red (stubbed accessors) and green; `receipt_survives_a_json_round_trip` was green from the start because the serde derive was added in the same edit as the test (not independently red).
- Real server smoke (container, port 8099): `/health` 200; a good `POST /decisions` returned a signed receipt and the chain head reached height 2; an output containing an SSN was refused with 422.
- Totals: 15 + 22 = 37 passing, 0 failed.
