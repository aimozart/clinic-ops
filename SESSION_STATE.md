# SESSION_STATE: clinic-ops

## 2026-10-17 05:26 PM: project AI home created
- Rust workspace: decisions (22 tests) + clinic-api (15 tests) green via ./run-cargo.sh test. Next: CI (WO1), container + deploy (WO2), scanning (WO3), clinic-mcp guardrail server, Entropa anchoring client. Mentor: Spock (claude-core-brain). Scott directs and audits.

## 2026-10-17: clinic-mcp architecture document (branch mcp-architecture, not pushed)
- docs/ARCHITECTURE-MCP.md holds the design with Scott's six decisions (schema, validate-only kql_query, PatientRef, phi_matches, clinic-api availability, rmcp pin after smoke test). No code, nothing pushed or merged.
- Next (each needs Scott's say): (1) rmcp smoke test in ./run-cargo.sh, pin version; (2) decisions::phi_matches test-first; (3) clinic-mcp policy engine; (4) KQL validator; (5) clinic-api client tools; (6) protocol tests. Execution of KQL against ADX is a later version and needs its own token-handling decision.
- Still unverified: see section 10 of the document.
