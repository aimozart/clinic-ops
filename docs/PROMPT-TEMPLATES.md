# Prompt templates (professional, reusable)

Good prompts for AI agents are **structured, specific and checkable**. Every template below uses the same seven parts, so any prompt can be reviewed quickly.

## The structure
1. **Goal**: one sentence, the outcome (not the method).
2. **Context**: what exists already (files, constraints, decisions). Point to files instead of pasting them.
3. **Constraints**: platform, versions, security rules, what must NOT happen.
4. **Steps** (only if order matters): numbered, small.
5. **Acceptance criteria**: how we will know it is done, as checks that can pass or fail.
6. **Evidence required**: the command and its output; what to say it could not verify.
7. **Output format**: files changed, a short summary, no merging or deploying.

## Why this works (prompt-engineering notes)
- **Zero-shot** (no example) is fine for simple, well-known tasks. **Few-shot** (one or two examples of the exact output you want) is better when format or style matters.
- **Context decides quality.** Put durable rules in `CLAUDE.md` / `copilot-instructions.md` (read every time) and task details in the prompt. Chat history is limited, so decisions that must survive belong in files.
- **Ask for evidence, not confidence.** "Show the failing test, then the passing test" beats "make sure it works".
- **Small, checkable tasks** beat one huge request. Review each result before the next.
- **Anti-patterns:** vague goals ("make it better"), pasting secrets, no acceptance criteria, letting the agent decide scope, accepting "done" without output.

## Template A: build a feature (test first)
```
Goal: <one sentence>.
Context: see crates/<crate>/src/<file>.rs and docs/AI-POLICY.md. <relevant decision>.
Constraints: Rust, tests via ./run-cargo.sh test; synthetic data only; no new dependencies without asking.
Steps: 1) write the failing tests, 2) show them failing for the stated reason, 3) implement, 4) show them passing.
Acceptance: <list of observable checks>.
Evidence: paste the red and green test output. List anything you could not verify.
Output: changed files + a 5-line summary. Do not commit, merge or push.
```

## Template B: review / audit AI-written output
```
Goal: audit <file or PR> as a skeptical reviewer.
Context: it was written by an AI agent. Rules: docs/AI-POLICY.md.
Check: parses and matches the platform schema; permissions least-privilege; versions pinned and quoted; no secrets; every referenced name exists; tests actually test something; human gates present.
Output: a table of findings (severity, location, why, fix). Say what you could not verify. Do not change files.
```

## Template C: debug a failure
```
Goal: find the cause of: <exact error text>.
Context: what changed last: <change>. Command that fails: <command>.
Constraints: read-only investigation first; propose the smallest fix.
Steps: reproduce, isolate, state the cause with evidence, then propose a fix.
Acceptance: the same command that failed now passes.
Evidence: exact outputs before and after.
```

## Template D: write infrastructure or a pipeline
```
Goal: <what the pipeline/Bicep must do>.
Context: target <Azure resource group / GitHub repo>; auth via OIDC; existing files: <paths>.
Constraints: minimal permissions; pinned versions; quote YAML versions; no anchors or merge keys; no secrets; cost <stated>.
Acceptance: parses; platform validation passes (<how>); what-if shows only the expected changes.
Evidence: validation output. Do not deploy.
```

## Template E: research with sources
```
Goal: answer <question> for <decision>.
Constraints: use official documentation; cite each claim with its URL; mark anything you could not confirm as UNVERIFIED.
Output: short answer first, then the evidence table (claim, source, confidence).
```

## Few-shot example (format control)
```
Summarise a CI failure in this exact format.
Example input: "npm ERR! test failed: expected 200 got 500 at /health"
Example output: "Cause: /health returned 500. Evidence: test log line. Next check: app startup logs."
Now do the same for: <new log text>
```
