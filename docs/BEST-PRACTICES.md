# Best practices (from the Paxel review of Scott's Entropa work, plus Oct 17 lessons)
1. **Plan, prove, prune.** Each change leaves evidence the decision worked; after growth, do a simplification pass (the one real Paxel finding: the codebase was additive-heavy).
2. **No fake evidence.** No placeholder CI, empty tests, "scan passed" notes without output, or docs for commands that do not run.
3. **Document the analyzer's blind spots next to the code** with a command anyone can run (Rust co-located tests are undercounted by tools that only count `tests/` files; public test vectors look like secrets).
4. **Decision-outcomes log.** Every consequential decision gets a dated line with command evidence and the result.
5. **Definition of Done** written down and checked, not assumed.
6. **Secret hygiene.** gitleaks before any public push; confirm hits against git history; document confirmed false positives.
7. **Test first, red before green.** Confirm the test fails for the stated reason before implementing.
8. **State the cost before anything billable; leave nothing running.**
9. **Rebuild from code.** Infrastructure is Bicep; a fresh deploy must work from the files alone.
