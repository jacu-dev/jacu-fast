# Jacu Fast

A workflow for coding agents, with an optional local evidence runner. **The skill helps the agent work; the runtime records what actually ran against which files.** Neither is an independent proof that every business requirement was understood.

## Choose a mode

| Mode | What you get | What it does not claim |
| --- | --- | --- |
| Skill only | A task-scoped procedure: inspect existing work, preserve requirements, implement, test and explain delivery. Works without a Jacu executable. | No Jacu receipts, cache, deterministic completion result or installed hooks. |
| Full Claude plugin | The same skill, a launcher, prebuilt runtimes, package diagnostics, and task-scoped SessionStart/Stop hooks. | No global permission bypass, daemon, model routing or automatic discovery of arbitrary business behavior. |

The 0.3.0 runtime fixes the unsafe 0.2.0 cache identity. **Do not reuse 0.2.0 receipts or the old binary.** Source and binary versions are checked before execution.

## Install in Claude Code

```text
claude plugin marketplace add https://github.com/jacu-dev/jacu-fast.git
claude plugin install jacu-fast@jacu-fast
```

Start a new **local Claude Code session**, including the Code tab in Desktop. The plugin skill is:

```text
/jacu-fast:jacu-fast implement the requested change and verify delivery
```

You may also ask in ordinary language: **“Use Jacu Fast for this task.”** Model invocation is enabled; the SessionStart hook explains the installed skill name. This makes discovery possible, not a guarantee that a probabilistic agent always selects it. The exact slash invocation is the unambiguous entry point.

`/jacu-fast` is the standalone skill name, not the namespaced plugin command. There is no separate `/jacu-fast:run` command. Installing this plugin does not install it in unrelated cloud chat sessions.

For updates, refresh the marketplace and plugin, then start a new session:

```text
claude plugin marketplace update jacu-fast
claude plugin update jacu-fast@jacu-fast
claude plugin list
```

The package-level diagnostic is `python3 PLUGIN_ROOT/scripts/host.py doctor`. Replace `PLUGIN_ROOT` with the installation path reported by Claude. It distinguishes “files present” from “loaded in this host session”; it cannot inspect a Desktop session from a separate process.

## Install only the skill

```sh
npx --yes skills add jacu-dev/jacu-fast -g -y --skill jacu-fast -a claude-code
```

The standalone invocation is `/jacu-fast`. The copy is useful **without a binary**. The agent must report **skill-only mode**, run the project's existing tools itself, and not invent a runtime completion result. Do not install both forms merely to work around a command-name mismatch.

Agent Skills-compatible hosts may reuse the procedure. The native event adapter in this repository is for Claude Code. A portable manifest does not mean Codex, OpenCode or another host's event integration was tested. There is no OpenCode npm plugin and no MCP server.

## What happens during a task

The agent captures the original request and a compact outcome contract, inspects branches and linked worktrees before duplicating work, and uses existing project conventions. With the runtime available, it prepares a session against the actual delivery directory/branch, runs useful checks at iteration checkpoints, and obtains current delivery evidence. A Stop hook is activated **only for that task and host session**.

The hook evaluates saved evidence; it never starts the test suite or a model call. Missing evidence sends a bounded recovery instruction to the same agent. An unchanged repeated stop or real terminal blocker ends with an explicit **incomplete** result instead of an approval prompt or infinite retry. Inactive Stop hooks do no repository work.

The runtime discovers existing Cargo, Go, Swift and common Node scripts when `.jacu-fast.json` is absent. Discovery does not install dependencies or invent missing tests. Unknown test output remains unknown. Use an explicit policy for project-specific runners or ignored fixtures.

## What the runtime verifies

- Content identity includes tracked and untracked file bytes, index state, file modes, symlink inputs, initialized submodules, and explicitly declared ignored inputs. It is not just the commit or Git status.
- A receipt is bound to the request, candidate, check definition, effective environment and executable/toolchain fingerprint. Failures are not success; a diagnosed transient failure can be retried explicitly.
- The delivery directory and branch are pinned. Required checks and already-bound outcome statements cannot be silently weakened or waived by a model-written amendment.
- Recognized zero-test runs, timeouts, truncated output and unsupported test counts do not pass. A semantic score needs its configured threshold and cannot replace behavioral evidence.

**Limits:** the agent still maps the request to outcomes and declares implementation state. Source quotes are checked when supplied, but coverage of an arbitrary natural-language request is not proven. Local receipts are mutable by the same OS user; this is not a tamper-proof attestation or security boundary against that user. Repositories and configured commands must already be trusted for execution. No general dependency-impact analysis, worktree-content search, automatic merge, assertion-semantic analysis or speedup benchmark is claimed.

## Runtime and development

Prebuilt targets: macOS Apple Silicon and Linux x86-64 built on Ubuntu 24.04 (glibc 2.39 or newer). The launcher and hooks require Python 3.9+; the compiled CLI itself has no Python dependency. macOS Intel, Linux ARM and Windows use skill-only mode unless a compatible developer runtime is explicitly supplied. No runtime download or compilation occurs silently during an agent task.

```sh
bin/jacu capabilities
cargo test --locked
cargo build --release --locked
JACU_TEST_BIN="$PWD/target/release/jacu" python3 -m unittest discover -s tests -p 'test_*.py' -v
JACU_BIN="$PWD/target/release/jacu" python3 scripts/host.py doctor
```

`JACU_BIN` is an explicit developer override, not a PATH fallback. Packaged builds instead require `runtimes/manifest.json`, a matching source fingerprint, binary version and checksum. Developer overrides are identified as such by the diagnostic; they are not claimed to have packaged-build provenance.

See [the runtime contract](skills/jacu-fast/references/runtime.md), [the engineering scope](docs/IMPLEMENTATION_PLAN.md), and [the acceptance evidence map](docs/acceptance-cases.json). CI tests the actual compiled CLI on Linux and macOS, the Python hook protocol, package consistency and an isolated Claude CLI installation. An authenticated end-to-end model session and your specific Desktop installation remain separate checks, not implied by unit-test success.

`prepare`, `verify`, `report`, `clean`, and `capabilities` are the native commands. `clean` only removes owned expired diagnostics for the selected repository; it does not delete branches, worktrees, build caches or receipts. Timing fields are measured operation timings, not claimed token or cost savings.

## License

MIT. Keep the copyright and license notice when redistributing. Contributions and public examples must be English and synthetic; do not publish credentials, private projects or raw user sessions.
