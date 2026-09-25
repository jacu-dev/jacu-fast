# Runtime contract — 0.3.0

The skill is independently usable. This contract applies only after the packaged launcher or an explicit developer runtime passes `capabilities`. Never use the old 0.2.0 executable for these guarantees.

## Agent lifecycle

Use an absolute path to the plugin's `bin/jacu`. Store request and contract files outside the target repository (for example, a task directory under `JACU_HOME`), so writing task metadata does not invalidate source snapshots. Commands take structured arguments, not interpolated request text.

```sh
bin/jacu prepare --repo /absolute/delivery/tree --request-file /task/request.txt --format json
bin/jacu prepare --repo /absolute/delivery/tree --request-file /task/request.txt --contract-file /task/contract.json --session SESSION --format json
bin/jacu verify --repo /absolute/delivery/tree --session SESSION --checkpoint iteration --format json
bin/jacu verify --repo /absolute/delivery/tree --session SESSION --checkpoint delivery --format json
bin/jacu report --repo /absolute/delivery/tree --session SESSION --format json
```

Supply a contract on the first prepare when it is already structured. Otherwise, bind it to the returned session. No two-call ritual is required. Preserve the original request bytes for all preparations in that session. Use `--audit` for a read-only task; keep that flag on subsequent preparations. Audit results are observations, not execution or correctness certification.

Preparation pins the canonical directory and its checked-out branch as the delivery target. Prepare against the **actual delivery worktree**, even if implementation occurs elsewhere. The agent must inspect and integrate relevant work using existing host tools. Jacu inventories worktree paths; it does not search their source for implementations or perform merges. Reusing a session after switching its branch cannot satisfy delivery.

## Contract example

For an original request containing exactly `The greeting must be Hello.`, a contract is:

```json
{
  "schema_version": 1,
  "outcomes": [{
    "id": "O1",
    "statement": "The greeting must be Hello.",
    "source_quote": "The greeting must be Hello.",
    "implementation": "pending",
    "delivery": "pending",
    "evidence": ["greeting-test"]
  }],
  "effort": {"uncertainty": 0, "novelty": 0, "coupling": 0, "consequence": "ordinary"}
}
```

Each evidence ID must exist in the effective policy. Update implementation to `present` and delivery to `in_target` only after observing those facts, then resubmit with the same `--session`. Already-bound statements, source quotes and evidence obligations cannot be removed or weakened. Additional outcomes and evidence are allowed. A source quote, when supplied, must occur in the original request. An optional `delivery_target` object (`path`, `branch`) must match the prepared directory and branch.

An initial `not_applicable` outcome requires a reason and is visibly agent-declared, not a runtime proof. An active outcome cannot later become inapplicable. Removing or weakening scope via a model-authored amendment is rejected. Legitimately changed user requirements belong to a separately authorized task; retain the old incomplete record rather than rewriting history. The runtime cannot independently authenticate the source of that authorization.

## Project policy

An explicit `.jacu-fast.json` is optional. If absent, existing root manifests are inspected: Cargo `test --locked`, Go `test -json ./...`, Swift `test`, and declared Node `typecheck`, `lint`, `test` scripts using the lockfile's package manager. Discovery is read-only and defaults to **no reuse**. No dependency installer runs. Prepare reports all discovered commands for the agent to inspect before executing in an already-trusted repository.

A minimal explicit policy:

```json
{
  "schema_version": 1,
  "checks": [{
    "id": "greeting-test",
    "kind": "external",
    "program": "./tests/greeting-check",
    "args": [],
    "cwd": ".",
    "timeout_seconds": 60,
    "minimum_tests": 0,
    "reuse": "never",
    "inputs": []
  }],
  "delivery": {"required_check_ids": ["greeting-test"]}
}
```

`kind` can identify `cargo-test`, `go-test`, `swift-test`, `node-test`, generic `test`, `semantic`, or a generic external/build check. A kind containing `test` requires a recognized positive test count even if `minimum_tests` is omitted. Cargo summaries are summed across suites; empty doctests do not cancel a nonempty suite. Go counts JSON test-level pass events. Swift aggregate summaries are not counted repeatedly. Unrecognized formats return unknown: use a project-owned adapter emitting a supported summary, not a model-written fake result. External checks prove only that their configured command succeeded.

`inputs` explicitly adds ignored fixture files or directories inside the repository to the snapshot. Do not list output/cache directories. `environment` is retained for compatibility; the runtime fingerprints the entire effective child environment without persisting values. Shell bookkeeping (`_`, `SHLVL`) is removed from children and fingerprints. CI/editor/pager variables are normalized consistently.

`reuse` is `never` or `same-session-declared-inputs`. Enable reuse only when the command's inputs are fully represented. Network, service, database, clock and other external state must use `external_state: true` or `reuse: never`. A passed receipt with non-reusable external state describes the last observed run, not perpetual service health. Local executable and recognized toolchain version fingerprints are included; this is not complete environmental hermeticity.

Use `verify --retry` after diagnosing a transient external failure or unknown result. It reruns selected checks without requiring a dummy source edit; it does not turn a failure into a pass or weaken requirements. There is no unbounded automatic retry.

A semantic command must output a JSON object with matching `request_sha256`, `evidence_id`, and a finite score between zero and one meeting `minimum_score` (default 0.8). No built-in Jev client, credentials or paid fallback exists. Semantic evidence alone cannot satisfy completion.

## Host binding and hooks

After preparing a real runtime task, bind it to Claude's actual session ID:

```sh
python3 PLUGIN_ROOT/scripts/host.py activate --repo DELIVERY_TREE --session JACU_SESSION --host-session CLAUDE_SESSION --workspace CLAUDE_WORKSPACE
```

Use Claude's supplied `${CLAUDE_SESSION_ID}` value; do not invent it or expose one task's gate to unrelated sessions. `--workspace` captures the Claude working directory when the delivery tree is a different linked worktree. Without an observable host ID, report that hook binding is unavailable and use explicit runtime verification.

The registered Stop adapter runs only `report`, never checks, builds or model calls. It translates runtime results to the host's `decision: block`/`reason` protocol, honors `stop_hook_active`, and bounds repeated attempts. Terminal failures or exhausted recovery produce an explicit incomplete message. Host permissions still apply; Jacu adds no approval question.

The legacy `verify --hook HOST` is evaluation-only CLI output, **not** a host adapter by itself. `scripts/host.py` owns the actual event translation. `doctor` checks package files and runtime capabilities, not whether a separate Desktop session loaded the plugin.

## Results and trust

Exit codes: `0` command success (`ready`, `complete`, or `assessment_complete`); `2` known check failure; `3` incomplete or missing required evidence; `4` invalid input/runtime state. Read JSON as well as the code. Ready is not complete.

Completion means: the agent-declared active outcomes have evidence, bound required commands have current passing receipts, no protected obligation was weakened, and the recorded delivery directory/branch is the one observed. `assurance` explicitly distinguishes those checks from agent-declared implementation and unproven coverage of the original request. Local state is not authenticated against an agent with the same OS write privileges. Tests themselves can still be inadequate; meaningful behavioral coverage remains essential.

Snapshots include tracked/untracked content, index state, modes, contained symlinks, initialized submodules and declared ignored inputs. Missing/inaccessible inputs, concurrent changes, process deadlines or essential output truncation cannot certify a pass. Snapshot file/time/depth bounds fail closed. This is not an atomic filesystem snapshot or a general impact-selection engine.

State is stored outside the project under `JACU_HOME` or `~/.jacu-fast`. Versions are pinned. Old sessions are rejected rather than silently upgraded. Operation timings are measured; token use and cost remain unknown. Disk counts cover fingerprinted inputs, not all ignored build/dependency directories. Cleanup affects only owned expired regenerable diagnostics for the selected repository, never worktrees, branches or receipts.
