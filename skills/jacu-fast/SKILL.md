---
name: jacu-fast
description: Use Jacu Fast for an explicitly requested development or workflow-audit task. Reuse existing work, calibrate effort, avoid redundant checks and deliver complete integrated outcomes with evidence, without Jacu approval prompts.
---

# Jacu Fast

Stay in the current agent session. Do not create or manage subagents, switch providers or change global permissions. Apply this workflow to the invoked task, not unrelated conversations.

The user invokes this skill knowing its workflow. Do not ask whether to continue, run checks, use fallback or integrate an already-permitted contribution. Brief progress updates are fine. A real blocker produces evidence and a truthful incomplete result, not a confirmation question.

## Start

Resolve the version-matched packaged `jacu` executable using the host's package path and supported-platform map. Never silently use an unrelated executable on PATH or compile/install missing tools. If the executable is unavailable, disclose that limitation; do not invent Jacu results.

Load [the runtime reference](references/runtime.md) for arguments and result semantics. Load only the relevant reference sections and project evidence, not the entire implementation plan.

Use `prepare` once per task to inspect the scoped repository and linked worktrees. Find existing relevant implementation before writing it again. Preserve unrelated, active, dirty or ambiguous work. Audit intent remains read-only.

Bind a compact outcome contract to the original user request/specification. Cross-check for omitted outcomes. Identify existing code to reuse, minimal changes, the delivery target and required checks. A small task needs a small contract, not a planning ceremony.

## Effort and implementation

Provide brief scores for uncertainty, novelty and coupling on 0–2 scales; track consequence separately. Use the returned effort recommendation only through supported host controls. Record unsupported effort changes as advisory. Reassess only when meaningful new ambiguity, risk, conflict or unexplained failure appears.

Implement the smallest sufficient behavior change in the repository's conventions. Prefer targeted patches and concise meaningful names. Do not generate speculative abstractions, redundant documents, opaque source aliases or unrelated refactors. Compact reference IDs may identify source in context; expand stale/ambiguous references before editing.

Lower effort and time pressure never reduce requested outcomes. Do not replace a required integration with a mock, remove a negative case, weaken an assertion or silently defer an outcome to finish faster.

## Verify and repair

Use `verify` at a coherent iteration checkpoint, not after every file save. Choose the next useful check from the task/project policy. Do not mechanically run every checker or the full suite. A focused TDD red/green cycle is compatible with this workflow.

Reuse only valid successful receipts. Preserve the original failure. A zero-test selection, timeout, obsolete snapshot, unavailable required check or semantic opinion is not passing evidence.

Repair the relevant behavior, then rerun the affected scope. Another round needs new evidence, a meaningful change or a specific unresolved hypothesis. Do not repeat identical failing work endlessly.

Use an already-configured JEV integration only for a bounded judgment likely to save more work than it costs. No JEV, timeout or disallowed data egress -> deterministic continuation. Never configure credentials, invoke a hidden paid substitute or use a score to waive required checks.

## Reconcile and finish

Integrate task-required contributions into the defined delivery target using permitted existing tools. Resolve relevant conflicts by inspection and verification; do not blindly choose a side or merge unrelated branches. Preserving work is not the same as delivering it.

Run delivery verification against the actual combined candidate. It accounts for every outcome, missing/invalidated evidence and relevant work outside the target. It must not rerun unchanged valid checks merely as a finishing ritual.

Use `report` for the final status. Report implementation, evidence and integration separately; include measured time and material limitations. Never claim completion from an unchecked list or the agent's own narrative.

On recoverable incompleteness, continue automatically. On an exhausted/external blocker, retain obligations and return an honest terminal incomplete/failed report without asking a question. Do not fabricate success or promise background work.
