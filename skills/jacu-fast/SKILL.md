---
name: jacu-fast
description: Use when the user explicitly asks for Jacu Fast on a development or workflow-audit task. This release is skill-only and has no jacu executable.
disable-model-invocation: true
---

# Jacu Fast

This release is skill-only and contains no Jacu executable. Do not search for, build, install, or run `jacu`. Apply the workflow with the tools and checks already available in the current project. Use those tool results as evidence. Do not invent Jacu receipts, timings, or a passing result.

`references/runtime.md` describes a future command-line contract. It is not an execution dependency of this release.

Stay in the current agent session. Do not create or manage subagents, switch providers, or change global permissions. Apply this workflow to the invoked task, not unrelated conversations.

The user invokes this skill knowing its workflow. Do not ask whether to continue, run checks, use a fallback, or integrate an already-permitted contribution. Brief progress updates are fine. A real blocker produces evidence and a truthful incomplete result, not a confirmation question.

## Start

Find an implementation that already exists before writing it again. Inspect the scoped repository and its linked worktrees with the host's existing tools. Preserve unrelated, active, dirty, or ambiguous work. An audit stays read-only.

Bind a compact outcome list to the original request. Cross-check for omitted outcomes. Identify code to reuse, the smallest sufficient change, the delivery target, and the checks that decision depends on. A small task needs a small list, not a planning ceremony.

## Effort and implementation

Score uncertainty, novelty, and coupling from 0 to 2, and record consequence separately. The sum is guidance for the work in this session: 0–1 focused, 2–4 standard, 5–6 deep. Sensitive or critical consequence raises that guidance and is not averaged away. There is no Jacu control that applies effort inside the host. Record the recommendation as advisory. Do not spawn an agent or edit global settings to simulate it.

Implement the smallest sufficient change in the repository's conventions. Prefer targeted patches and concise names. Do not add speculative abstractions, redundant documents, or unrelated refactors.

Lower effort never removes a requested outcome. Do not replace a required integration with a mock, drop a negative case, weaken an assertion, or leave an outcome unstated in order to finish sooner.

## Verify and repair

Run the next useful project check at a coherent checkpoint, not the full suite after every save. Choose that check from the task and the project's own instructions. A focused red/green test cycle fits this workflow.

Reuse an earlier result only when you still have its real output and the inputs it depends on are unchanged. A failure stays a failure. Zero selected tests, a timeout, a truncated result, an obsolete snapshot, or an unavailable required check is not a pass.

Repair the relevant behavior, then rerun the affected check. Another round needs new evidence, a meaningful change, or a specific unresolved hypothesis.

Use an already-configured JEV integration only for a bounded judgment that is likely to save more work than it costs. If JEV is missing, slow, or disallowed, continue with the project's own checks. Do not configure credentials, call a hidden paid substitute, or let a model score waive a required check.

## Reconcile and finish

Put the task's required changes in the delivery target with the tools the host already permits. Resolve a relevant conflict by inspection and a real check. Preserving work elsewhere is not the same as delivering it.

Before calling the work complete, account for every outcome, missing evidence, and relevant work outside the target. Do not rerun an unchanged check that already has a valid result.

Report implementation, evidence, and integration separately. Include measured time and material limitations. Never claim completion from an unchecked list.

On a recoverable gap, continue. On an exhausted or external blocker, keep the unmet outcome and return an incomplete or failed result without asking a question. Do not fabricate success or promise background work.
