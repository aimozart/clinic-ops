# Testing
Tests are co-located (`#[cfg(test)] mod tests`), the idiomatic Rust layout. A scanner that only counts files under `tests/` will undercount (see the same note in entropa-public ANALYSIS_NOTES.md).

    ./run-cargo.sh test

Red/green record (2026-10-17): with stub functions, 0 passed and 20 failed ("not yet implemented"); after implementation, 20 passed, 0 failed.
