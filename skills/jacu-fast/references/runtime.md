# Jacu Fast Runtime Reference

Version 0.2.0 ships `bin/jacu` for macOS Apple Silicon. The commands below are the contract for that binary. Release packaging must keep this reference aligned with the actual CLI. The same source is tested on Linux by CI. This package does not contain a Linux or Windows executable.

## Command lifecycle

```sh
jacu prepare --repo PATH --request-file PATH --format json
jacu prepare --repo PATH --request-file PATH --audit --format json
jacu prepare --repo PATH --request-file PATH --contract-file PATH --session ID --format json
jacu verify --repo PATH --session ID --checkpoint iteration --format json
jacu verify --repo PATH --session ID --checkpoint delivery --format json
jacu report --repo PATH --session ID --format json
```

Use the packaged absolute executable path. Write request/contract payloads through existing host file tools. Session records go under `JACU_HOME`, or `~/.jacu-fast` when that variable is unset. They are not written into the target repository. Never interpolate task text into a shell command.

The first preparation may inventory context before the current agent submits a compact contract for the same session. Skip redundant preparation when the request is already structured. `verify` selects outstanding checks from the known policy, not arbitrary model-generated commands.

## Results

Every result carries schema version, session ID, observed candidate identity, outcome, evidence references, pending obligations, next actions, terminal state and measured/unknown timings.

`ready` is not delivered. `assessment_complete` is a completed audit, not a correctness certification. `complete` requires all active outcomes, valid mandatory evidence and reconciliation in the delivery target. `incomplete`, `failed` and `unavailable` remain truthful states.

Check states: `passed`, `failed`, `reused`, `not_applicable`, `deferred`, `unknown`. Iteration deferral does not waive delivery requirements. Reuse applies only to valid successful receipts. A prior failure remains failed even when rerunning it is unnecessary.

Normal exits: 0 command success; 2 known check failure; 3 incomplete or required unavailable evidence; 4 invalid input/configuration or runtime failure. Read JSON as well as exit status. Hook adapters use their host-specific protocol, not these exit codes blindly.

## Effort

Score uncertainty, novelty, coupling 0–2 each. Initial sum bands: 0–1 focused, 2–4 standard, 5–6 deep. Sensitive/critical consequence imposes a floor and is not averaged into a low score. Unknown context is not evidence of a trivial task.

Recommended effort is portable guidance. Applied effort must be confirmed by a supported host interface or recorded as null/advisory. No subagents, silent model switching or global setting edits.

## Invalid evidence

Zero tests; truncated essential output; timeout; stale source; changed dirty/untracked inputs; changed features, toolchain, fixtures, policy or relevant environment; stateful external services without a reuse contract; forged/imported receipts represented as native execution.

## Non-interactivity

No approval questions, credential setup, interactive installer or broad deletion confirmation. Execute in-scope available work automatically. Preserve unknown data automatically. Return focused recovery to the current agent. Exhausted/external blockers terminate honestly without a question or infinite retry.
