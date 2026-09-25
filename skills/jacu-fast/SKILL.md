---
name: jacu-fast
description: Use when the user explicitly asks for Jacu Fast on a development or workflow-audit task. This release runs the packaged jacu executable.
disable-model-invocation: true
---

# Jacu Fast

This release includes the `jacu` executable at `bin/jacu` in the plugin. Use that packaged file. Do not use a different `jacu` from `PATH`, and do not compile or download another one.

Resolve the binary in this order: `JACU_BIN` when it is an absolute path to a file, then `$CLAUDE_PLUGIN_ROOT/bin/jacu`, then `bin/jacu` two directories above this skill. This packaged build is macOS Apple Silicon (`darwin` / `arm64`). On any other platform, report the binary as unavailable and stop. Do not invent receipts, timings, or a passing result.

`references/runtime.md` is the command contract for this binary.

Stay in the current agent session. Do not create or manage subagents, switch providers, or change global permissions. Apply this workflow to the invoked task, not unrelated conversations.

The user invokes this skill knowing its workflow. Do not ask whether to continue, run checks, use a fallback, or integrate an already-permitted contribution. Brief progress updates are fine. A real blocker produces evidence and a truthful incomplete result, not a confirmation question.

## Start

Write the original request to a file. Call:

```sh
jacu prepare --repo REPO --request-file REQUEST --format json
```

Add `--audit` when the task is read-only. For an audit, do not edit the project. `assessment_complete` is not a certification.

When the result is `ready` and the outcome contract is missing, write a contract file and call `prepare` again with `--contract-file` and `--session` set to the returned session id. Bind every requested outcome. Do not drop an outcome unless the contract includes an amendment that names it.

Score uncertainty, novelty, and coupling from 0 to 2, and record consequence separately. The executable returns `recommended_effort` as advisory. It does not change host settings. There is no Jacu control that applies effort inside the host.

## Effort and implementation

The sum is guidance for the work in this session: 0–1 focused, 2–4 standard, 5–6 deep. Sensitive or critical consequence raises that guidance and is not averaged away.

Find an implementation that already exists before writing it again. Inspect the repository and its linked worktrees with the host's existing tools. Preserve unrelated, active, dirty, or ambiguous work.

Implement the smallest sufficient change in the repository's conventions. Prefer targeted patches and concise names. Do not add speculative abstractions, redundant documents, or unrelated refactors.

Lower effort never removes a requested outcome. Do not replace a required integration with a mock, drop a negative case, weaken an assertion, or leave an outcome unstated in order to finish sooner.

## Verify and repair

Project checks come from `.jacu-fast.json` in the repository root. Jacu runs only those argument arrays. It does not turn model text into a shell command.

```sh
jacu verify --repo REPO --session SESSION --checkpoint iteration --format json
```

Run the next useful check at a coherent checkpoint, not the full suite after every save. A focused red/green test cycle fits this workflow.

Reuse applies only when the executable reports the same candidate and a still-valid receipt. A failure stays a failure. Zero selected tests, a timeout, a truncated result, or an unknown result is not a pass. An unchanged candidate does not rerun a failed or unknown check.

Repair the relevant behavior, then call `verify` again. Another round needs new evidence, a meaningful change, or a specific unresolved hypothesis.

Before calling the work complete:

```sh
jacu verify --repo REPO --session SESSION --checkpoint delivery --format json
```

`ready` is not delivered. `complete` requires every active outcome, valid required evidence, and the recorded delivery target.

Use an already-configured JEV integration only for a bounded judgment that is likely to save more work than it costs. If JEV is missing, slow, or disallowed, continue with the project's own checks. Do not configure credentials, call a hidden paid substitute, or let a model score waive a required check.

## Reconcile and finish

Put the task's required changes in the delivery target with the tools the host already permits. Resolve a relevant conflict by inspection and a real check. Preserving work elsewhere is not the same as delivering it.

```sh
jacu report --repo REPO --session SESSION --format json
```

Report implementation, evidence, and integration separately. Include the measured timings from the executable. Never claim completion from an unchecked list.

Exit status: 0 ready, complete, or assessment complete; 2 a known check failed; 3 incomplete or required evidence unavailable; 4 invalid input. Read the JSON as well as the exit status.

On a recoverable gap, continue. On an exhausted or external blocker, keep the unmet outcome and return an incomplete or failed result without asking a question. Do not fabricate success or promise background work.
