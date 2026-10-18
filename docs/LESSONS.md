# Lessons learned (keep honest; add dated lines)
- 2026-10-17: Read the key-facts and ALL restored memory before asking Scott anything; he already told us. Search notes first, ask last.
- 2026-10-17: Scott's statements about his own work are fact. Do not hedge or ask him to prove them. Typos: read charitably.
- 2026-10-17: Check roles BEFORE a lab (Contributor cannot write role assignments; Owner needed).
- 2026-10-17: Check which account the CLI is signed in as (`az account show`) before any change.
- 2026-10-17: A tool that sends a request twice can create phantom errors (our script sent every call twice; the 409 was ours). Test tools against a safe endpoint first.
- 2026-10-17: In KQL, columns built from lookup lists are `dynamic` and cannot be grouped; converting a dynamic column with `.alter column` emptied the data. Load with `tostring()`.
- 2026-10-17: YAML: platforms differ (Azure accepted `yes`, GitHub rejected it; GitHub rejected `if: !cancelled()`; unquoted `3.10` became 3.1 in a GitHub matrix). The published parser is not the live service: test on the real one.
- 2026-10-17: Verify counts by running them (docs said 93/72/207; reality was 106 public and 229 private).
- 2026-10-17: Cost: read-only look at GCP showed about $6/day from an idle Kubernetes setup; look before assuming.
