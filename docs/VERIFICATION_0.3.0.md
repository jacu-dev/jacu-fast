# 0.3.0 remediation and verification

Date: 2026-09-25. Baseline: `ae6c4b1f4ac9c424c971f25f73724bd21e839566` (0.2.0).

## Delivered scope

The portable skill is useful without the optional executable. The full Claude Code plugin packages the same skill, a Python-standard-library host adapter, task-scoped SessionStart/Stop hooks and source-bound native executables. The existing agent still performs implementation and Git integration. No daemon, model router, MCP service, credential setup or additional approval workflow was added.

The canonical plugin entry is `/jacu-fast:jacu-fast`. Standalone skill copies use `/jacu-fast`. Model invocation is enabled, and SessionStart explains the namespaced entry. Natural-language discovery is supported, not guaranteed selection by every model. A missing runtime is reported as skill-only mode rather than silently abandoning the task.

## Correctness changes

- Candidate identity uses actual tracked/untracked file content, index state, modes, contained symlinks, initialized submodules and declared ignored inputs. Editing an already-dirty or already-untracked file invalidates prior evidence.
- Receipts bind the original request, candidate, effective check definition, executable/toolchain and effective child environment. Shell bookkeeping is normalized. PWD is explicitly set to the canonical command directory, so a shell launcher and a direct completion hook observe the same effective execution inputs.
- Original request bytes, session version and already-bound obligations cannot be silently replaced. Source quotations are checked when supplied. Model-authored amendments cannot waive protected scope or weaken required checks.
- The delivery directory and branch are observed and pinned. A task cannot complete merely because an agent changed an `in_target` string after switching to another branch.
- Recognized zero-test runs, unknown required test counts, timeouts and truncated output do not pass. Semantic responses must meet their threshold and cannot replace behavioral evidence. Diagnosed transient failures have an explicit bounded retry path.
- Child execution has closed stdin, bounded output and process-group deadlines, including descendants that retain output pipes after the parent exits. Cleanup is restricted to owned expired diagnostics for the selected repository.
- Missing project policy can discover existing Cargo, Go, Swift Package and common Node checks. Discovery is read-only, does not install dependencies, and disables reuse by default.

## Executed verification

The build source fingerprint is `ad44cc7512994ea32c4f1bfa9e2630d79dd23974967993fd1188cce4960f79f1`. Both native executables in `runtimes/manifest.json` were built from that exact formatted Rust source with Rust 1.98.1. The manifest records individual executable SHA-256 values and build provenance.

The two **test jobs** in [build run 36144234810](https://github.com/jacu-dev/jacu-fast/actions/runs/36144234810) succeeded on Linux x86-64 / Ubuntu 24.04 and macOS 15 / Apple Silicon. Each executed:

| Layer | Observed result |
| --- | --- |
| Rust unit, acceptance and protocol tests | 66 passed: 3 unit, 54 acceptance and 9 protocol. |
| Python black-box and host integration tests | 39 passed, including the actual compiled executable rather than a reimplementation. |
| Formatting and package checks | Formatted source and manifest/skill consistency checked. |
| Native release executable | Built, fingerprinted, packaged and executed on its own target platform. |
| Real Claude Code 2.1.282 validation | Strict marketplace, plugin-manifest and skill-directory validation passed. |
| Real clean Claude plugin installation | Installed into an isolated configuration directory, listed as enabled version 0.3.0 and inspected with `plugin details`. |
| Installed runtime | `doctor` reported runtime mode and the launcher inside Claude's installed cache executed `capabilities` successfully. |

That is **105 automated tests per platform**, plus package and real host-installation checks. These counts are not a coverage percentage, a speed benchmark, or a claim that arbitrary requirements are completely understood.

The original build run's final publishing job was denied permission to change workflow files using its restricted Actions token. Its two test jobs and artifact assembly succeeded. The exact assembled commit was subsequently published using the authorized repository connection, without changing its source or binaries. The normal `validate` workflow then validates the committed package; do not confuse the earlier publishing permission failure with a passing all-jobs run.

### Cross-component regression

`tests/test_real_hook.py` drives the real packaged launcher, actual Rust runtime and actual Python Stop adapter together. It starts with an intentionally stale inherited PWD, records a task, binds a host session, observes a blocking Stop result before verification, verifies the behavior, changes the stale caller PWD, confirms that unchanged effective inputs reuse the receipt, then observes a released Stop result without a second test execution. No runtime result is mocked.

This test failed against the earlier candidate, exposed a launcher/hook environment mismatch, and passed after normalizing the actual child environment. The final Linux artifact was additionally exercised outside CI with the same integrated flow.

## Explicit limits

An authenticated interactive Claude/model session and the user's particular Desktop installation were **not** exercised. The real CLI test proves installation, registration and execution from the installed package; protocol and integration tests prove the implemented event contract. They do not prove that every live model will choose the skill. Use the exact namespaced command for unambiguous invocation and start a new session after updating.

The agent maps the original request to outcomes and declares implementation state. The runtime does not independently prove arbitrary natural-language coverage, adequacy of test assertions, or complete business semantics. Local receipts are not tamper-proof against the same operating-system user. Repositories and configured commands must already be trusted.

Worktree inventory is not semantic source search or an automatic Git merge. Those tasks remain explicit responsibilities of the skill-driven agent. No universal test-impact engine, unattended worktree deletion, cross-host event compatibility, token savings or measured development speedup is claimed.

Linux executables target glibc 2.39 or newer. The packaged native targets are macOS Apple Silicon and Linux x86-64. Other environments retain the portable skill-only workflow; the launcher and Claude adapter require Python 3.9 or newer. The compiled runtime itself does not depend on Python.

## Updating

```text
claude plugin marketplace update jacu-fast
claude plugin update jacu-fast@jacu-fast
claude plugin list
```

Start a new local Claude Code session and invoke `/jacu-fast:jacu-fast` followed by the task. Do not reuse 0.2.0 receipts or substitute its binary. Missing runtime or hook binding must be reported explicitly; neither means that a check passed.
