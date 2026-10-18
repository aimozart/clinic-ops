# ARCHITECTURE-MCP: Clinic Ops read-only guardrail server (`clinic-mcp`)

Status: **design only, no code yet** (revised 2026-10-17 with Scott's six decisions). This is a proposal for Scott and Spock to audit. Anything not confirmed is marked **UNVERIFIED** and collected in section 10.

## 1. Purpose and shape

`clinic-mcp` is a Model Context Protocol server that lets an AI assistant *ask* the Clinic Ops system questions and *check* its own intended actions against policy, without being able to change anything.

- Language: Rust, new workspace crate `crates/clinic-mcp`.
- SDK: `rmcp` **>= 1.4.0**. The exact version is pinned only after a compile smoke test (section 8, step 1).
- Transport: **stdio only**. No HTTP listener, no port.
- Data: **synthetic only** (L3). No secrets in config, arguments, logs or tool output (L4).
- Tools: exactly four, all read-only: `check_action`, `kql_query`, `verify_receipt`, `list_decisions`.
- Version 1 never connects to the KQL cluster and never spawns a process (section 4.1).

```
AI client (spawns process)
   |  stdin/stdout: newline-delimited JSON-RPC (MCP)
   v
clinic-mcp --- policy engine          (pure, deterministic, no I/O)
   |       --- KQL validator          (pure, no I/O, v1 is validate-only)
   |
   |-- GET  /decisions, /chain/head  -->  clinic-api (existing, synthetic)
   '-- POST /verify (read-only check) -->  clinic-api
```

Why it talks to `clinic-api` instead of embedding a `Ledger`: the ledger in `crates/clinic-api/src/lib.rs` lives in memory (`Ledger::new(Probe::spawn(), now)` at startup). A second process holding its own `Ledger` would have a different key and chain, so receipts would never verify. `clinic-mcp` calls only `GET /decisions`, `GET /chain/head` and `POST /verify`. `POST /verify` uses POST but does not mutate the chain (`Ledger::verify_receipt` takes `&self`). `POST /decisions` is never called.

### 1.1 clinic-api availability

- `verify_receipt` and `list_decisions` need `clinic-api`. If it cannot be reached (connection refused, timeout, non-2xx, unparseable body) they return a tool execution error (`isError: true`) whose structured content is `{ "error": "unavailable", "detail": "clinic-api not reachable" }`. The detail never includes request payloads.
- The server **never starts, restarts or supervises** `clinic-api`. Starting it is a human action.
- `check_action` and `kql_query` need no backend and keep working when `clinic-api` is down.
- **The ledger is in memory and empties when `clinic-api` restarts**, with a new key (`Probe::spawn()`). After a restart: `list_decisions` returns `total: 0`, `chain/head` shows a new head and public key, and every receipt issued before the restart verifies as `valid: false`. That `false` is correct, not a fault. The tool descriptions must say so, and `verify_receipt` returns the current `pubkey`, so a caller can see the key changed.

## 2. Policy model

Every request is classified into exactly one verdict. Default is **deny**.

| Verdict | Meaning | Who acts |
|---|---|---|
| `allow` | Read-only and inside the allowlist. | Server answers. |
| `ask` | Not forbidden, but a human gate is required (AI-POLICY "Human gates"). The server does **not** perform it; it returns the reason and what Scott must approve. | Scott, outside this server. |
| `deny` | Forbidden or unrecognised. Refused, reason returned, event logged to stderr (verdict and rule id only). | Nobody. |

Rule order (first match wins): **deny rules, then ask rules, then allow rules, else deny**.

### 2.1 Action classes used by `check_action`

| Class (`action` string) | Verdict | Reason |
|---|---|---|
| `read_file`, `list_files`, `run_tests`, `git_status`, `git_diff`, `git_log` (inside repo) | allow | Read-only checks (AI-POLICY "May do"). |
| `write_file`, `edit_file`, `git_commit` (on a branch) | allow | Drafting is permitted; commits follow L7 and the commit-identity rule. The server only reports the verdict. |
| `open_pull_request` (on a branch) | allow | AI-POLICY "May do". |
| `merge`, `deploy`, `push_to_main` | ask | Human gates; L5. |
| `force_push` | deny | L5. |
| `create_cloud_resource`, `change_billing`, `scale_up` | ask | Billable; L5 and the global cloud-credit rules. The reason must say "state the cost and how it is turned off first". |
| `upgrade_account`, `remove_spending_limit`, `change_tenant`, `change_offer_type`, `close_account` | deny | Global rules 1 and 2: could void the credit. Never allowed, not even by `ask`. |
| `delete_*` (any delete, destroy, uninstall, wipe) | deny | Global DELETING rule: the server cannot satisfy "exact word plus separate yes/no", so it never approves a delete. |
| `read_secret`, or a path matching `~/.config/secrets`, `.env`, token or key files | deny | L4. |
| `use_real_patient_data`, or input matching the PHI patterns | deny | L3. |
| `weaken_test` (delete or loosen an assertion to pass) | deny | AI-POLICY. |
| `change_policy` (edits to `docs/AI-POLICY.md` or this server's rules) | ask | Human gate: "any change to this policy". |
| anything else | deny | Default deny. |

Classification is a lookup table plus path and pattern rules in code. It is **not** an LLM call. The same input always gives the same verdict, the same property as `decisions::checker::check`.

### 2.2 PHI pattern check (shared, not copied)

The `decisions` crate's `phi_patterns()` (SSN, email, phone, MRN) is private today. Decision: expose it as a public function and reuse it. **Do not copy the regexes into `clinic-mcp`.**

- Proposed API in `crates/decisions/src/checker.rs`: `pub fn phi_matches(text: &str) -> Vec<&'static str>` returning the pattern names that match. `check()` is refactored to call it, so there is one source of truth. (Name and signature are a proposal.)
- Test first (L2): in `decisions`, write failing tests that `phi_matches` returns `["ssn"]` for a synthetic SSN-shaped string, returns `[]` for clean text, can return several names, and that `check()` still returns `Violation::PhiPattern(..)` for the same inputs (the existing 22 tests must stay green). Show them red for the stated reason (function missing), then green.
- Used by `check_action` (`target`, `context`) and by `kql_query` (string literals in the query, rule K7).

## 3. Tools

All four tools are intended to carry the `readOnlyHint` annotation, but the exact field in the pinned `rmcp` version is **UNVERIFIED**, and the MCP spec says clients treat annotations as untrusted. The real guarantee is the server code (section 7). Results carry `structuredContent` conforming to a declared `outputSchema`, plus the same JSON as a text block (spec: SHOULD, for compatibility). Tools are listed in a fixed order (spec: SHOULD be deterministic): `check_action`, `kql_query`, `verify_receipt`, `list_decisions`.

Error mapping (spec, both revisions read): a policy refusal is a **normal result** (`isError: false`, `verdict: "deny"`), because the refusal is the answer. Unknown tool or malformed arguments are **protocol errors** (JSON-RPC `-32602`). Backend problems are **tool execution errors** (`isError: true`).

### 3.1 `check_action`

Asks "would this action be allowed under project policy?" Performs nothing.

- **Input**
  - `action` (string, required, max 64 chars): one of the classes in 2.1; free text is classified as unknown, so deny.
  - `target` (string, optional, max 512 chars): path, resource name or branch.
  - `context` (string, optional, max 1000 chars): the AI's stated reason.
- **Output** `{ verdict: "allow"|"ask"|"deny", rule: string, reason: string, needs: string|null }`. `rule` is the id of the matching table row; `needs` names the human gate for `ask`.
- **Refuses**
  - Anything not in the table (deny).
  - A `target` outside the repo root, containing `..`, or matching the secret-path patterns (deny, L4).
  - A `target` or `context` for which `phi_matches` is non-empty (deny, L3).
  - Strings over the length limits (protocol error).
- Needs no backend.

### 3.2 `kql_query` (version 1: VALIDATE-ONLY)

Checks a KQL query against the allowlist in section 5 and returns a verdict. **It never runs the query, never connects to the Azure Data Explorer cluster, holds no token, and returns no data.** The tool description must say exactly that, so a model does not expect rows.

- **Input** `query` (string, required, max 2000 chars). Nothing else: there is no `max_rows`, `timespan` or connection parameter in v1.
- **Output** `{ verdict: "allow"|"deny", rule: string, reason: string, normalised_query: string|null, executed: false }`
  - `rule`: `"K0"` when every rule passed, otherwise the id of the first failed rule (K1 to K9, section 5.1).
  - `normalised_query`: set only when `verdict` is `allow`, else `null`. Normalisation is deterministic: comments removed, whitespace collapsed to single spaces with one pipe per line, and `| take 50` appended if the query has no `take`/`limit`/`top`. It must be **idempotent** (normalising a normalised query changes nothing) and an allowed query must still be allowed after normalisation.
  - `executed` is always `false` in v1; the field exists so a later execution version cannot be mistaken for this one.
- **Refuses** every query that fails a rule in 5.1, including multiple statements, control commands (leading `.`), tables outside the allowlist, unknown columns, a missing or wrong time filter, and direct use of `PatientRef`.
- **Does not need `clinic-api`.** It has no network code path at all. The validator lives in a module with no HTTP, socket, process or file dependency.
- **Execution is a later version** and needs its own decision about token handling. Until then, no component of `clinic-mcp` may hold an Azure token, connection string or key.

### 3.3 `verify_receipt`

Checks that a ledger receipt is genuine: that block exists, with that content hash, signed by that key.

- **Input** a `Receipt` exactly as in `crates/decisions/src/ledger.rs`: `index` (u64), `content_hash`, `block_hash`, `signature`, `pubkey` (strings). Hex strings only, each length-capped. The ML-DSA-65 signature is large, so the cap must come from a real receipt (**UNVERIFIED**: exact length).
- **Output** `{ valid: bool, chain_verified: bool, height: u64, pubkey: string }`. `valid` comes from `clinic-api` `POST /verify`; `chain_verified`, `height` and `pubkey` from `GET /chain/head`.
- **Refuses**
  - Non-hex or oversize fields, and extra fields (schema sets `additionalProperties: false`): protocol error.
  - It never signs, never creates a receipt and never accepts a private key.
  - `valid: false` means "not found or does not match", which is not an error. After a `clinic-api` restart, old receipts are `false` (section 1.1).
  - `clinic-api` unreachable: `unavailable` error (section 1.1).
- A valid receipt proves who recorded what and that it was not changed. It does not prove the AI decision was correct (AI-POLICY note). The tool description says so.

### 3.4 `list_decisions`

Lists recorded AI decisions (metadata only).

- **Input** `limit` (integer, 1 to 100, default 20), `kind` (optional, one of `review`, `triage`, `kql_explain`, the checker's `ALLOWED_KINDS`), `since_index` (optional u64).
- **Output** `{ decisions: [{ index, actor, kind, block_hash, timestamp }], total: u64 }`, from `GET /decisions` (which already skips genesis and returns only those five fields). Filtering by `kind`, `since_index` and `limit` is done in `clinic-mcp`.
- **Refuses**
  - Unknown `kind` and out-of-range `limit` (protocol error; never silently honoured).
  - It cannot return `output` text or `input`: `clinic-api` does not list them, and `input` is never stored (only its blake3 hash). The server must not add a way to fetch them.
  - `clinic-api` unreachable: `unavailable` error (section 1.1).

## 4. Data source: Azure Data Explorer (for later execution, and for validation now)

- Cluster: an Azure Data Explorer **free cluster**, database **`kql-phxdb`**.
- v1 uses this only as the **schema the validator checks against**. No connection is made.
- There is **no `TimeGenerated`** column in any allowlisted table. Each table has its own time column.

### 4.1 Allowlisted tables and columns

| Table | Time column | Columns |
|---|---|---|
| `ClinicVisits` | `VisitTime` | `VisitId`, `PatientRef`, `Clinic`, `Department`, `Insurance`, `VisitTime`, `Status`, `WaitMinutes` |
| `AccessAudit` | `Timestamp` | `Timestamp`, `Role`, `Action`, `ResourceType`, `Success` |
| `AIDecisions` | `Timestamp` | `ChainIndex`, `Timestamp`, `Actor`, `Model`, `HumanGate`, `LatencyMs`, `Violations` |

Column data types and the set of values in `Status`, `Action`, `Role`, `HumanGate` and `Violations` are not recorded here (**UNVERIFIED**). String literals in the examples below are illustrative.

## 5. KQL allowlist (`kql_query`)

KQL is validated by a small hand-written tokenizer and parser in the server, not by regex over the raw string alone. Reject on any parse doubt (fail closed). The first failed rule decides the rule id.

### 5.1 Rules (all must hold)

| Id | Rule |
|---|---|
| K1 | Exactly one statement: no `;`, query length <= 2000 chars, nesting depth <= 4. |
| K2 | The first token is one of `ClinicVisits`, `AccessAudit`, `AIDecisions`. |
| K3 | Only these operators after the table: `where`, `project`, `project-away`, `summarize`, `top`, `take`/`limit`, `order by`/`sort by`, `extend`, `distinct`, `count`. `render` is not allowed. |
| K4 | Only these functions: `count`, `countif`, `dcount`, `sum`, `avg`, `min`, `max`, `percentile`, `bin`, `ago`, `now`, `tostring`, `toint`, `tolong`, `strlen`, `iff`, `isnull`, `isempty`. (`tostring()` is listed because `docs/LESSONS.md` records that dynamic columns must be converted this way before grouping.) |
| K5 | **Time filter on the table's own time column** (`VisitTime` for `ClinicVisits`, `Timestamp` for `AccessAudit` and `AIDecisions`). The first `where` must contain, as a top-level `and` conjunct (never under `or`), `<TimeColumn> > ago(N)` or `>=`, with N in `m`, `h` or `d` units and at most 7 days. A time filter on any other column, or `TimeGenerated`, fails K5. |
| K6 | A row limit: `take`/`limit`/`top` with a value <= 200. If absent, normalisation appends `| take 50` (it never appends a time filter). |
| K7 | Banned anywhere, case-insensitive, including inside strings and comments: `.` control commands (`.set`, `.append`, `.ingest`, `.drop`, `.alter`, `.create`, `.delete`), `externaldata`, `http_request`, `http_request_post`, `evaluate`, `plugin`, `invoke`, `union`, `join`, `lookup`, `cluster(`, `database(`, `external_table`, `materialize`, `print`, `sql_request`, `cosmosdb_sql_request`, `toscalar`. Also banned: any string literal for which `decisions::phi_matches` is non-empty (section 2.2). |
| K8 | Every column name must be a column of the table in K2 (section 4.1) or a name defined earlier in the same query by `extend`, `summarize` or `project`. Anything else (for example `Name`, `SSN`, `TimeGenerated`) is denied. |
| K9 | **`PatientRef` minimisation.** `PatientRef` may appear only as the argument of `dcount()` (or `count`-family aggregates that accept a column; see section 10). It must not be projected, grouped by (`by PatientRef`), listed with `distinct`, ordered or topped by, used in `extend`, or compared in `where`. The rule also catches `project PatientRef` renamed through `project X = PatientRef`. |

### 5.2 Allowed examples (real tables; literal values illustrative)

```kql
ClinicVisits
| where VisitTime > ago(7d) and Status == "NoShow"
| summarize visits = count(), avg_wait = avg(WaitMinutes) by Clinic
| top 10 by visits desc
```
```kql
ClinicVisits
| where VisitTime > ago(7d)
| summarize patients = dcount(PatientRef) by Department
| take 50
```
```kql
AIDecisions
| where Timestamp > ago(24h)
| summarize decisions = count(), p95_latency = percentile(LatencyMs, 95) by Model
```
```kql
AccessAudit
| where Timestamp > ago(1d) and Success == false
| summarize failures = count() by Role, Action
| take 50
```

### 5.3 Denied examples

| Query | Rule |
|---|---|
| `.set AIDecisions <\| print 1` | K2 (first token is not a table); K7 also applies (control command, `print`). |
| `.drop table ClinicVisits` | K2, K7. |
| `ClinicVisits \| where VisitTime > ago(1d) \| join AccessAudit on Role` | K7 (`join`). |
| `AIDecisions \| union *` | K7 (`union`). |
| `externaldata(x:string) [h@"https://example.invalid/x"]` | K2, K7. |
| `AIDecisions \| where Timestamp > ago(1d) \| evaluate http_request("https://example.invalid")` | K7. |
| `SecurityEvent \| take 10` | K2 (table not allowlisted). |
| `ClinicVisits \| take 10; AccessAudit \| take 10` | K1 (two statements). |
| `ClinicVisits \| where VisitTime > ago(365d) \| take 5` | K5 (window over 7 days). |
| `ClinicVisits \| take 5` | K5 (no time filter). |
| `ClinicVisits \| where TimeGenerated > ago(1d)` | K5 (wrong time column), K8 (no such column). |
| `AIDecisions \| where VisitTime > ago(1d)` | K5/K8 (`VisitTime` belongs to `ClinicVisits`). |
| `AccessAudit \| where Timestamp > ago(1d) or Success == false` | K5 (time filter under `or`). |
| `ClinicVisits \| where VisitTime > ago(1d) \| project Name \| take 5` | K8 (unknown column). |
| `ClinicVisits \| where VisitTime > ago(1d) \| project PatientRef, Clinic \| take 5` | K9. |
| `ClinicVisits \| where VisitTime > ago(1d) \| project P = PatientRef \| take 5` | K9 (rename). |
| `ClinicVisits \| where VisitTime > ago(1d) \| summarize n = count() by PatientRef` | K9 (group by). |
| `ClinicVisits \| where VisitTime > ago(1d) \| distinct PatientRef` | K9. |
| `ClinicVisits \| where VisitTime > ago(1d) and PatientRef == "x" \| take 5` | K9 (compared in `where`). |
| `ClinicVisits \| where VisitTime > ago(1d) and Clinic == "123-45-6789" \| take 5` | K7 (SSN-shaped literal, via `phi_matches`). |

## 6. Runtime and protocol requirements

Sources are in section 9.

- **stdout is protocol only.** Both spec revisions: the server MUST NOT write anything to stdout that is not a valid MCP message; logs go to stderr. A stray `println!` is a protocol bug. Add a test for it (for example `clippy::print_stdout` denied in the crate; **UNVERIFIED** that this lint suits the setup).
- **Exit when stdin closes.** The newest stdio page says servers SHOULD exit promptly on stdin EOF.
- **Cancellation.** On `notifications/cancelled` the server should stop work and send nothing further for that request. All four tools are fast and pure or single-call, so this is trivial.
- **Capabilities:** `tools` only (with `listChanged` false). No resources, prompts, sampling, roots or elicitation. The server never needs input from the client; `ask` is an ordinary result, not an elicitation.
- **Validation:** every input is checked against its JSON Schema and again in Rust (length, range, charset). The spec says servers MUST validate all tool inputs, implement access controls, rate limit and sanitise outputs. Plan: a per-process rate limit (a number to be agreed, for example 60 calls/min) and PHI-pattern screening of anything returned.
- **Human in the loop:** the spec says there SHOULD be a human who can deny tool invocations. That is the client's job; `ask` hands the decision back to that human.
- **Config:** one environment variable, `CLINIC_API_URL` (default loopback). No keys, tokens or passwords exist in the design. A backend timeout (for example 3 s) is a proposal.

### 6.1 MCP spec revisions compared

I read the Tools and Transports pages of revision **2025-06-18** (first draft) and of **2026-07-28** (this revision, including the stdio binding page). The rmcp README says the SDK implements 2026-07-28 and stays compatible with 2025-11-25 and earlier.

| Topic | 2025-06-18 | 2026-07-28 | Effect on this design |
|---|---|---|---|
| Handshake | `initialize` session | No connection session; every request carries `_meta.io.modelcontextprotocol/protocolVersion`, `clientInfo` and `clientCapabilities`; `server/discover` probe for compatibility | The server holds no per-connection state. Handled by rmcp, not by us (how rmcp does it is **UNVERIFIED**). |
| Tool result | `content`, `isError`, `structuredContent` (object) | Adds `resultType: "complete"`; `structuredContent` may be any JSON value | Our outputs are objects, valid in both. |
| Tool list | no ordering rule | `tools/list` SHOULD be deterministic in order; set MUST NOT vary per connection; adds `ttlMs`, `cacheScope`, `icons` | Fixed order (section 3). |
| Tool names | free | SHOULD be 1 to 128 chars of `A-Za-z0-9_-.` | Our four names comply. |
| Server-initiated requests | allowed | Servers do not initiate JSON-RPC requests; stdio server MUST NOT write requests to stdout; input needs use `InputRequiredResult` | We never ask the client anything. |
| stdio framing and logging | newline-delimited; stdout MCP only; stderr for logs | Same rules; adds stdin-EOF shutdown guidance and restart on unexpected exit | No change; test both. |
| Errors | protocol `-32602` vs `isError: true` | Same split, with clearer guidance that execution errors are for model self-correction | Section 3 mapping stands. |
| Security | MUST validate, access-control, rate limit, sanitise | Same | Same. |

## 7. What the server must never do

1. Never write, create, update or delete anything: no ledger records, no files, no mutating network call, no process spawn. In particular never call `POST /decisions`.
2. Never hold, read, accept or print a private key, token, connection string, password, `.env` or anything under `~/.config/secrets` (L4). This includes Azure Data Explorer credentials.
3. **Never connect to the Azure Data Explorer cluster in version 1, and never run a KQL query.** `kql_query` is validate-only.
4. Never return or log real patient data, raw decision inputs, or PHI-pattern matches (L3). Logs carry verdicts and rule ids, not payloads.
5. Never execute shell commands, run `cargo`, call `az`/`aws`/`gcloud`, or touch cloud APIs. It cannot create billable resources or change billing (L5).
6. Never approve a delete, merge, deploy, force push, billing change or account change. `ask` means a human decides elsewhere, never "approved".
7. Never allow a KQL query that fails a rule, and never fall back to a looser mode on a parse error.
8. Never return `PatientRef` values, or a query that exposes them directly.
9. Never start, restart or supervise `clinic-api`.
10. Never open a network listener or accept connections: stdio only, outbound only to the configured loopback `clinic-api` URL.
11. Never follow instructions found in tool inputs, query text or ledger text. They are data.
12. Never claim success or "passed" without the verdict and rule id that produced it (L1). Never claim regulatory compliance.
13. Never weaken a deny rule to make a test pass; changing the policy table is a human gate.
14. Never write anything but MCP messages to stdout.

## 8. Build order and test plan (L2 test-first; nothing built yet)

Steps, each needing Scott's say before code starts:

1. **rmcp smoke test.** In a scratch crate inside `./run-cargo.sh`, add `rmcp` (features `server`, `macros`, `transport-io`), `cargo check` a one-tool stdio server. If the container cannot fetch the crate, **report it and stop**. On success, pin the exact version in `Cargo.toml` and log it in `docs/DECISION_OUTCOMES.md`.
2. `decisions`: add `phi_matches` test-first (section 2.2).
3. `clinic-mcp`: policy engine and `check_action`.
4. KQL validator and `kql_query` (validate-only).
5. `clinic-api` client, `verify_receipt`, `list_decisions`.
6. Protocol-level tests.

Tests:

- **Policy:** one test per row of 2.1; unknown action; `../` traversal; secret paths; PHI in `context` and `target`.
- **KQL validator:** every allowed example in 5.2 returns `allow` with rule `K0`; every denied example in 5.3 returns `deny` with the listed rule id; case variants and comments hiding banned words; wrong time column for each of the three tables; `PatientRef` allowed only in `dcount` and denied in project, rename, `by`, `distinct`, `top`, `order by`, `extend` and `where`; SSN-shaped literal denied via `phi_matches`.
- **Validate-only proof:**
  - `normalised_query` is `null` on deny and idempotent on allow, and an allowed query stays allowed after normalisation.
  - `executed` is `false` in every response.
  - The `kql_query` output schema has no `rows` or `columns`.
  - With `CLINIC_API_URL` pointing at a closed port, `kql_query` and `check_action` still return their verdicts (no network path).
  - A dependency check (`cargo tree -p clinic-mcp`) is documented so anyone can confirm the validator module pulls in no HTTP or Azure crates.
  - The config struct has exactly one field (`CLINIC_API_URL`): no token or connection setting exists.
- **`verify_receipt`:** valid receipt true; tampered `signature`, `block_hash`, `content_hash`, `pubkey`, `index` each false (mirrors existing ledger tests); a receipt from before a simulated `clinic-api` restart is false.
- **`list_decisions`:** limit clamping and rejection, unknown `kind`, no `output` or `input` field present.
- **Unavailable:** with `clinic-api` down, `verify_receipt` and `list_decisions` return `isError: true` with `error: "unavailable"`, and the server does not try to start anything.
- **Protocol:** spawn the binary, run the handshake that the pinned rmcp supports, `tools/list` returns exactly four tools in the fixed order, `tools/call` for each, every stdout line is valid JSON-RPC, stdin EOF makes the process exit.
- Run through `./run-cargo.sh test`.

## 9. Documentation used

Fetched 2026-10-17:

- rmcp crate page (version, feature flags `server`, `macros`, `transport-io`; no quickstart there): https://docs.rs/crate/rmcp/latest. Latest shown: 3.5.1, released 2026-10-05.
- Official Rust SDK README (tool macros, `serve(stdio())`, protocol versions): https://raw.githubusercontent.com/modelcontextprotocol/rust-sdk/main/README.md
- MCP spec 2025-06-18, Tools: https://modelcontextprotocol.io/specification/2025-06-18/server/tools
- MCP spec 2025-06-18, Transports: https://modelcontextprotocol.io/specification/2025-06-18/basic/transports
- MCP spec 2026-07-28, Tools: https://modelcontextprotocol.io/specification/2026-07-28/server/tools
- MCP spec 2026-07-28, Transports overview: https://modelcontextprotocol.io/specification/2026-07-28/basic/transports
- MCP spec 2026-07-28, stdio binding: https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio
- Repo code read: `crates/decisions/src/{record,checker,ledger}.rs`, `crates/clinic-api/src/lib.rs`.
- Table, column and database names in section 4 come from Scott (decision "schema").

## 10. What I could not verify (still open)

- **rmcp API and version.** No code was compiled. Macro names (`tool_router`, `tool`, `Parameters`), the `stdio()` path and the feature set come from the README and the docs.rs crate page only. Whether the `./run-cargo.sh` container can fetch `rmcp` is untested (build step 1). "1.4.0 or newer" was not checked against a changelog; I only saw 3.5.1 as latest.
- **How the pinned rmcp handles the 2026-07-28 stateless protocol** versus the older `initialize` handshake, and which revision it negotiates by default. I did not read the 2026-07-28 lifecycle, versioning or `server/discover` pages.
- **The `readOnlyHint` annotation.** Neither tools page I fetched named that field (they only say `annotations` are optional and untrusted). Whether rmcp exposes it, and under what name, is unchecked.
- **KQL specifics.** The validator grammar (comments, string escapes, `let`, `by` clauses, `project` renames) was not checked against the Kusto documentation. Whether `count()` accepts a column argument (I believe it takes none, which would leave `dcount(PatientRef)` as the only way to use `PatientRef` under K9) is unchecked.
- **Column data types and value sets** for `Status`, `Action`, `Role`, `HumanGate`, `Violations` and the other columns. The example literals are illustrative.
- **Exact ML-DSA-65 signature and key hex lengths**, so the `verify_receipt` field caps are undetermined.
- **Numbers I proposed without testing:** the rate limit, the backend timeout, the `take 50` default, and whether the `clippy::print_stdout` lint fits.
- **Client behaviour.** I did not test how any AI client displays `ask` or `deny` results.
