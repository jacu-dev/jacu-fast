---
name: jacu-fast
description: Use Jacu Fast when the user asks for Jacu Fast, jacu-fast, use jacu, or a Jacu workflow audit. Complete the requested development task with explicit outcomes, current evidence, and delivery reconciliation. Supports natural-language invocation and a portable skill-only mode.
---

# Jacu Fast

Stay in the current agent session. Do not create another agent, switch providers,
edit global permissions, or ask for routine confirmation. A real blocker ends as
incomplete with its evidence, not as a question or an invented pass.

## Resolve the installed mode once

Claude plugin invocation is `/jacu-fast:jacu-fast <task>`. The standalone skill is
`/jacu-fast <task>`; other hosts use their own skill invocation syntax. A user's
natural-language request to use Jacu is also an instruction to invoke this skill.
Do not silently ignore it because a short slash alias is absent.

When the plugin is available, use its absolute path, not an unrelated `jacu` on
PATH. Resolve the root from `${CLAUDE_PLUGIN_ROOT}` or two directories above this
skill. Run `python3 "PLUGIN_ROOT/scripts/host.py" doctor` once. Never load the old
0.2.0 binary, fabricate capabilities, install credentials, or compile on demand.

When `mode` is `runtime`, use `PLUGIN_ROOT/bin/jacu` below. When the package,
Python adapter, or a compatible runtime is absent, continue automatically in
**skill-only mode**. State that mode once. It has no Jacu receipts, automatic
closure gate, reliable cross-call cache, or measured runtime savings. Use the
host's existing tools to carry out the same task; do not treat missing optional
infrastructure as a reason to abandon the requested work.

## Capture, investigate, implement

Keep a compact list mapping **every requested result to its source**. Include
negative cases and integrations, not just happy-path code. Cross-check it against
the original request. A generated list does not prove that nothing was omitted.
Do not discard an item to finish earlier. Separate implementation, evidence,
and delivery; preserve a visible pending item for any genuine limitation.

Inspect the current Git state and linked worktrees before duplicating work. Read
relevant diffs/files in other branches or worktrees; listing their paths is not
finding an implementation. Preserve unrelated, dirty, active, or ambiguous work.
Choose the smallest sufficient change using the project's own conventions.

In runtime mode, read `references/runtime.md` for exact JSON examples. Save the
request and contract outside the target repository. Call `prepare --repo REPO
--request-file REQUEST --contract-file CONTRACT`. Capture the returned session.
For a read-only audit add `--audit`; do not edit the target project or run its
commands. Select an explicit `delivery_target` when the requested destination is
not the current branch/worktree. The default is the observed current path/branch.

After preparation in Claude, bind the task to the actual host session:

```sh
python3 "PLUGIN_ROOT/scripts/host.py" activate --repo REPO --session SESSION --host-session "${CLAUDE_SESSION_ID}"
```

Use the real host session ID substituted by Claude. If it is unavailable or remains
literal, report that the automatic gate is unavailable and retain explicit final
verification; do not invent an ID. Other hosts do not inherit the Claude hook.

## Verify, repair, reconcile

Run useful checks at coherent checkpoints, not the whole suite after every save.
In runtime mode use `verify --repo REPO --session SESSION --checkpoint iteration`.
The runtime reads `.jacu-fast.json`, or discovers existing Cargo, Go, Swift Package
and package.json checks. Discovery never installs dependencies or writes policy.
For custom projects, derive commands from actual project scripts and write the
policy before binding a contract. Do not invent a passing placeholder command.

Reuse only the runtime's valid evidence. In skill-only mode, do not infer cache
validity from an unchanged commit or a remembered passing result. Zero tests,
unknown formats, timeout, truncated output and unverified state are not passes.
Fix behavior, then verify again. `--retry` is only for a diagnosed transient or
external recovery, not an endless unchanged-failure loop.

Use existing Jev only for a bounded assessment that adds value. No credential
setup, paid substitute, model score that waives tests, or mandatory Jev gate.
Effort guidance is advisory; do not claim host effort settings were changed.

Integrate the required work into the actual requested destination using the
host's permitted tools. Reconcile relevant branches/worktrees without deleting
unknown work. Preserved work elsewhere is not a delivered feature.

Before finishing, runtime mode requires `verify --repo REPO --session SESSION
--checkpoint delivery`, then `report --repo REPO --session SESSION`. `ready` is
not complete. In skill-only mode perform and name the actual final checks with
the host tools; distinguish observed results from claims that remain unverified.

Report the operating mode, implemented outcomes, executed checks, actual delivery
target and remaining limitations. Do not call an unrun test passed or a synthetic
protocol test a successful installation in the user's application. No promise
of background work. No routine request for the user's OK.
