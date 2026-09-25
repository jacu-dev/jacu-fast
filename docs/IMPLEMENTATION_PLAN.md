# Engineering scope — 0.3.0

This document supersedes the historical 0.1 plan, whose history remains in Git. It describes implemented interfaces and their limits, not a blanket claim that every acceptance case passed in every host.

## Product boundary

Jacu is a portable workflow skill plus an optional deterministic local runner. Keep one Cargo package, a thin Python-standard-library host adapter, one canonical skill and native host manifests. No daemon, service, MCP server, agent manager, permission wizard or paid evaluator is introduced. Public text and synthetic fixtures are English.

The native runner owns content identity, bounded command execution, receipts, immutable bound obligations and a pinned delivery directory/branch. The existing agent owns requirement interpretation, discovering relevant implementations, code changes, semantic judgment, integration and communicating blockers. A configured check is only as useful as its assertions. Agent declarations are never labeled independent proofs.

## Modules

`src/lib.rs` preserves the public prepare/verify/report contract, policy discovery, persistent state, contract validation, completion calculation and conservative diagnostics cleanup. `src/inventory.rs` fingerprints actual inputs with bounded Git/filesystem reads. `src/runner.rs` supervises child process groups and classifies bounded outputs. `src/main.rs` handles CLI arguments. `scripts/host.py` validates package provenance, selects a native runtime and implements task-scoped Claude event translation. It does not duplicate Rust verification logic.

## Reuse and completion

Content hashes, not porcelain status, identify candidates. Check definitions, effective environment and executable/toolchain identity participate in receipt matching. Source modifications during a check invalidate success. Conservative whole-candidate invalidation is intentional; no test-impact analysis or measured speedup is claimed. Explicit external-state checks do not reuse results during verification.

Original request bytes and session version are immutable. Already-bound outcome statements, evidence and required check definitions cannot be self-waived. An initial source quotation can be matched to the request, but arbitrary natural-language coverage cannot be established by Rust. A new authorized scope belongs to a new task rather than rewriting the previous one.

The delivery directory/branch is explicit and observed, including when it is a linked worktree. Jacu does not merge branches or search other worktrees for implementations; the skill makes the agent perform and report that investigation. Local receipts are not tamper-proof against the same OS user.

## Installation and release

The plugin command is `/jacu-fast:jacu-fast`; standalone skill installs use `/jacu-fast`. Natural-language selection is enabled, and the SessionStart hook describes the entry. A missing/incompatible runtime is an explicit skill-only mode rather than a dead-end installation.

Release automation builds and tests macOS arm64 and Linux x86-64, packages the exact binaries with source and binary hashes, and validates the launcher. A stale source/binary combination is refused. Linux artifacts built on Ubuntu 24.04 target glibc 2.39+. Other platforms retain the portable skill, without a claimed native event/runtime test.

Tests must distinguish the compiled CLI, host protocol fixtures, isolated real Claude CLI installation, and an authenticated interactive model/Desktop session. A test that merely finds a string in Markdown does not establish installation or skill invocation. The acceptance map records this distinction; CI logs are execution evidence, not static `passed` strings.

## Non-interactivity and failure

Closed stdin, explicit argument arrays, bounded streams, process groups and deadlines prevent Jacu-originated approval waits. An external denial remains a blocker, never a reason to modify global permissions. A Stop hook reads evidence without running the suite and bounds repeated recovery. Finishing incomplete with named obligations is different from discarding them or claiming completion.

## Acceptance and measurement

Retain and strengthen the original Rust protocol/acceptance tests. Add black-box regressions for already-dirty and already-untracked edits, executable/environment changes, pinned target changes, policy weakening, immutable requests, zero tests, semantic thresholds and descendants retaining pipes. Package tests reject stale/corrupt binaries. Hook fixtures demonstrate event protocol behavior, not a real model choosing the skill.

Evaluate workflow value separately on matched tasks: no skill, skill-only, and skill plus runtime. Measure time to correctly integrated delivery, check invocations, rework and escaped failures. No throughput or token-saving claim is made until such a controlled evaluation is performed. Keep cleanup, effort guidance and evaluator integrations secondary to reliable evidence and usable installation.
