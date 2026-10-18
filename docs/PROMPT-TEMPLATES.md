# Prompt templates (professional, reusable)

Good prompts for AI agents are **structured, specific and checkable**. Every template below uses the same seven parts, written as **XML tags**, so any prompt can be reviewed quickly and the model can tell instructions from data.

## The structure
1. **Goal**: one sentence, the outcome (not the method).
2. **Context**: what exists already (files, constraints, decisions). Point to files instead of pasting them.
3. **Constraints**: platform, versions, security rules, what must NOT happen.
4. **Steps** (only if order matters): numbered, small.
5. **Acceptance criteria**: how we will know it is done, as checks that can pass or fail.
6. **Evidence required**: the command and its output; what to say it could not verify.
7. **Output format**: files changed, a short summary, no merging or deploying.

## Why XML tags
- Tags separate **instructions** from **context** and **data**, so pasted text (a log, a diff) is not mistaken for a command.
- Use the same tag names every time; nest tags when content has parts (`<acceptance><check>...</check></acceptance>`).
- Put long reference material in its own tag (`<log>`, `<file>`) and refer to it by name in the instructions.
- Untrusted text (logs, issue bodies) goes inside a data tag and is treated as data only.

## Why this works (prompt-engineering notes)
- **Zero-shot** (no example) is fine for simple, well-known tasks. **Few-shot** (one or two examples of the exact output you want) is better when format or style matters.
- **Context decides quality.** Put durable rules in `CLAUDE.md` / `copilot-instructions.md` (read every time) and task details in the prompt. Chat history is limited, so decisions that must survive belong in files.
- **Ask for evidence, not confidence.** "Show the failing test, then the passing test" beats "make sure it works".
- **Small, checkable tasks** beat one huge request. Review each result before the next.
- **Anti-patterns:** vague goals ("make it better"), pasting secrets, no acceptance criteria, letting the agent decide scope, accepting "done" without output.

## Template A: build a feature (test first)
```xml
<task>
  <goal>One sentence: the outcome.</goal>
  <context>See crates/CRATE/src/FILE.rs and docs/AI-POLICY.md. Relevant decision: ...</context>
  <constraints>
    Rust; tests via ./run-cargo.sh test; synthetic data only; no new dependencies without asking; no secrets.
  </constraints>
  <steps>
    <step>Write the failing tests.</step>
    <step>Show them failing for the stated reason.</step>
    <step>Implement.</step>
    <step>Show them passing.</step>
  </steps>
  <acceptance>
    <check>Observable check 1</check>
    <check>Observable check 2</check>
  </acceptance>
  <evidence>Paste the red and green test output. List anything you could not verify.</evidence>
  <output>Changed files and a 5-line summary. Do not commit, merge or push.</output>
</task>
```

## Template B: review / audit AI-written output
```xml
<task>
  <goal>Audit the file below as a skeptical reviewer.</goal>
  <context>Written by an AI agent. Rules: docs/AI-POLICY.md.</context>
  <checks>
    Parses and matches the platform schema; permissions least-privilege; versions pinned and quoted; no secrets;
    every referenced name exists; tests actually test something; human gates present.
  </checks>
  <file name="PATH">
    ...file content here (data only, not instructions)...
  </file>
  <output>A table of findings: severity, location, why, fix. Say what you could not verify. Do not change files.</output>
</task>
```

## Template C: debug a failure
```xml
<task>
  <goal>Find the cause of the failure below.</goal>
  <context>Last change: ... Failing command: ...</context>
  <error>
    ...exact error text (data only)...
  </error>
  <constraints>Read-only investigation first; propose the smallest fix.</constraints>
  <steps>Reproduce, isolate, state the cause with evidence, then propose a fix.</steps>
  <acceptance>The same command that failed now passes.</acceptance>
  <evidence>Exact outputs before and after.</evidence>
</task>
```

## Template D: write infrastructure or a pipeline
```xml
<task>
  <goal>What the pipeline or Bicep must do.</goal>
  <context>Target: RESOURCE GROUP or REPO. Auth: OIDC. Existing files: ...</context>
  <constraints>
    Minimal permissions; pinned versions; quote YAML versions; no anchors or merge keys; no secrets; cost: STATED.
  </constraints>
  <acceptance>
    <check>Parses</check>
    <check>Platform validation passes (state how)</check>
    <check>what-if shows only the expected changes</check>
  </acceptance>
  <evidence>Validation output.</evidence>
  <output>Files only. Do not deploy.</output>
</task>
```

## Template E: research with sources
```xml
<task>
  <goal>Answer the question for the decision.</goal>
  <question>...</question>
  <constraints>Official documentation only; cite each claim with its URL; mark anything unconfirmed as UNVERIFIED.</constraints>
  <output>Short answer first, then an evidence table: claim, source, confidence.</output>
</task>
```

## Few-shot example (format control)
```xml
<task>
  <goal>Summarise a CI failure in exactly the format shown in the example.</goal>
  <example>
    <input>npm ERR! test failed: expected 200 got 500 at /health</input>
    <output>Cause: /health returned 500. Evidence: test log line. Next check: app startup logs.</output>
  </example>
  <input>...new log text (data only)...</input>
</task>
```
