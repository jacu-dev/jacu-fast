# Jacu Fast — Final Implementation Plan

**Plan version:** 1.0 · **Date:** September 24, 2026  
**Status:** 0.2.0 implements this plan's release behavior. `cargo test` is the evidence. The prebuilt executable in the repository is macOS Apple Silicon.  
**Repository:** https://github.com/jacu-dev/jacu-fast · **Visibility:** public · **License:** MIT  

The public repository contains the skill, this plan, the acceptance checklist, and the Rust `jacu` executable for macOS Apple Silicon. The local executor handoff is not part of this repository.

**Product version to implement first:** `0.1.0`  
**Source, documentation, comments, schemas, examples, issues and CLI output:** English  
**Runtime:** Rust · **Primary interface:** one installable plugin and one entry skill  
**Cargo package:** `jacu-fast` · **Plugin-local executable:** `jacu`

## 0. Authority, scope and reading order

This document replaces the normative design in the v0.1 research proposal and v0.2–v0.4 design reviews. Those documents remain research history, not additional requirements to combine indiscriminately. In particular, no earlier approval queue, mandatory JEV gate, general-purpose impact engine or source-obfuscation proposal survives unless explicitly retained here.

The executor must implement the release scope below, create the new public repository, validate the installable package and publish truthful results. Creating a repository or writing documentation alone is not implementation completion. This handoff package itself contains no compiled executable, working hooks or claimed benchmarks.

Use this plan as the engineering source of truth. Use the compact skill during ordinary product operation; do not load this entire plan into every task. `EXECUTOR_PROMPT.md` is a handoff instruction, not a new runtime agent.

License choice is a packaging default selected here, not a claim that the user previously selected MIT. Retain third-party notices where applicable. Never expose a private predecessor to make the new project public.

## 1. Product definition

**Jacu Fast minimizes the time an existing coding agent takes to deliver a complete, verified, integrated change.** It reduces unnecessary code, repeated exploration, redundant execution, disproportionate model effort and abandoned work.

It does not optimize the time until an agent claims success. An omitted requirement, deferred test, unmerged contribution or later repair remains part of the cost.

The user installs the plugin, invokes its entry skill with a task and does not operate intermediate commands. The current agent keeps implementing, investigating and integrating; Jacu supplies deterministic state, execution and closure checks. No supervisor, subagent manager, model router or independent coding agent is introduced.

The product is convention-preserving: it does not impose a new architecture, formatter, testing ideology or programming style on a target repository. It optimizes the agreed workflow, not the user's deliverables. Jacu's own repository is English and Rust; audited applications need not be.

### 1.1 Non-negotiable requirements

| ID | Requirement | Acceptance anchor |
|---|---|---|
| R01 | Create a new public `jacu-dev/jacu-fast`; preserve predecessors unchanged. | S0, repository inspection |
| R02 | Keep runtime logic in a small Rust package, with a thin executable and library. | S1, dependency/size report |
| R03 | Distribute a plugin with one entry skill and prebuilt executables. | S5, clean-install tests |
| R04 | Never introduce a Jacu-originated approval prompt, wizard or waiting-for-OK state. | NI acceptance family |
| R05 | Work without JEV, model credentials or a new agent runtime. | SEM acceptance family |
| R06 | Reuse existing project tools; verify coherent changes rather than every edit. | V acceptance family |
| R07 | Prevent duplicate verification when the evidence contract remains valid. | V04–V09 |
| R08 | Preserve all requested outcomes; no undisclosed shortcuts or self-approved deferrals. | C acceptance family |
| R09 | Find task-related work before reimplementing it and reconcile the delivered candidate. | G acceptance family |
| R10 | Score tasks and recommend/apply supported effort without spawning agents. | E acceptance family |
| R11 | Reduce context and generated code without opaque source renaming or a new language. | CT acceptance family |
| R12 | Support new projects, ongoing development and read-only audits of existing projects. | S2, audit tests |
| R13 | Include existing smoke, E2E and native-app checks in the same verification mechanism. | V11, S3 |
| R14 | Inventory worktree/build waste; implement narrowly scoped automatic cleanup in the named follow-on slice. | H acceptance family, S7 |
| R15 | Measure accepted-delivery time, total verification cost, rework and escapes. | M acceptance family |
| R16 | Publish English-only, sanitized materials and honest compatibility/benchmark claims. | P acceptance family |

### 1.2 What "non-interactive" means

Within the task's scope and the host's existing capabilities, perform routine actions automatically. Missing JEV uses deterministic fallback. Unknown or unrelated work is preserved automatically. A recoverable failure returns concrete next actions to the same agent. An exhausted or external failure produces a terminal `incomplete` or `failed` result, not a question and not a fabricated pass.

Installation and invocation express intent to use the documented workflow; do not repeatedly reconfirm that intent. They are not blanket permission to bypass operating-system controls, publish unrelated private data or destroy unknown work.

No Jacu approval UI, permission ledger requiring user clicks, `--yes` mode, `WAITING_FOR_APPROVAL` status, credential wizard, hidden paid fallback or instruction to ask the user to continue is allowed. External host permissions may still apply. Neither a skill nor a plugin can erase those controls. [S03, S04]

## 2. Delivery scope: small first release, explicit follow-ons

### 2.1 Required for the first usable release, 0.1.0

One Cargo package; `prepare`, `verify`, `report`; one primary skill; compact task contracts; local receipts; task-scoped inventory; minimal effort rubric; meaningful context references; one tested Cargo adapter; a bounded generic adapter for existing commands; completion reconciliation; optional existing-JEV bridge; read-only hygiene/audit; a tested Claude Code plugin distribution; a portable plugin package validated in an available local Codex-compatible host.

A host cannot be listed as operationally supported without an actual installation and invocation test. If a required host is unavailable to the executor, report that acceptance item as unverified and release only a clearly labeled preview for the tested host. Do not declare the complete multi-host milestone finished.

The first prebuilt platform is macOS Apple Silicon. Add Linux x86-64 as the second release target for CLI/CI validation. macOS Intel, Linux ARM64 and Windows are explicit follow-ons, not implied support. Use synthetic fixtures on all public CI jobs.

### 2.2 Explicitly scheduled after the first measured core

- **S7:** conservative automatic cleanup of Jacu-owned regenerable resources; preserve all ambiguous or unrelated data, without confirmation prompts.
- **X1:** tune slow Cargo and CI paths identified by measurement; compare existing runners/caches before replacing tools.
- **X2:** browser exploration and bounded replay repair through an existing runner, with optional JEV.
- **X3:** aggressive test-substitution experiments, including a JEV-only laboratory arm.
- **X4:** richer compact-context experiments only if they improve total delivery time.

These are named backlog items, not silently omitted requirements. Their experiment is delivered by running it and reporting a decision, even if the proposed optimization performs poorly. Promotion into the default workflow requires evidence. A 0.1 release cannot claim it already provides automatic worktree deletion, learned test selection or autonomous browser repair.

### 2.3 Keep out of the product

No daemon, SaaS, dashboard, database server, vector index, custom language/compiler, mandatory AST engine, plugin marketplace service, MCP server, subagent framework, model catalog, separate browser implementation or general Git merge engine.

Use the host and project tools already present. Small static shell command strings in host manifests and ordinary CI commands are packaging glue; product decisions, parsing, supervision and policy remain Rust. Do not grow Node/Python runtime services to distribute a Rust CLI.

## 3. User experience and lifecycle

Preferred portable invocation is `/jacu-fast <task>` where a host supports it. In Claude Code the native plugin entry will be `/jacu-fast:run <task>` because plugin skills are namespaced. A standalone compatibility alias may expose the shorter form only without overwriting an existing user command. It must be generated from the same skill, not maintained as a second workflow. [S01]

Codex/other hosts use their actual skill discovery syntax. Installing a skill in a cloud chat does not grant it access to a developer's local filesystem. Local execution support is the initial target, not universal desktop automation. [S04]

```text
Invoke the plugin with a task
  -> prepare once: inspect existing work and capture outcomes
  -> current agent: select proportional effort and implement a coherent slice
  -> verify iteration: next useful evidence, not every available check
  -> current agent: repair or integrate related work
  -> verify delivery: outstanding obligations and actual combined candidate
  -> report facts: complete, failed or incomplete
```

Audit intent remains read-only: report opportunities and evidence limits rather than implementing them automatically. Development intent permits in-scope changes through the already-authorized host. Missing project information uses observed conventions and explicit assumptions; if a consequential ambiguity cannot be resolved from available evidence, preserve the obligation and return its limitation without opening a question.

The skill must emit only useful progress notices and the final result. Do not ask whether to run tests, continue, merge a permitted contribution or use a deterministic fallback.

## 4. Minimal architecture

```text
Existing coding agent + one skill
             |
             v
Rust library / plugin-local jacu executable
  contract + policy
  Git/task inventory
  bounded command execution
  receipts + completion calculation
  reporting + measured timings
             |
  Git / Cargo / existing checks / optional JEV
```

Use one library and executable in one Cargo package. Ordinary modules for `contract`, `inventory`, `policy`, `runner`, `receipt` and `report` are sufficient; names may be adjusted to reduce duplication. A tested platform-specific process helper is acceptable. Do not split modules into crates or invent a plugin trait hierarchy without a real need.

Start with the standard library and a few established crates for CLI parsing, serialization and hashing. Pin the toolchain and commit `Cargo.lock`; record the MSRV. Resolve current compatible versions and advisories during implementation rather than copying stale versions from this plan. A blanket dependency-count limit must not encourage homemade cryptography or unsafe process control.

Resolve the executable from the plugin package, not arbitrary `PATH` lookup. The older `jacu` executable must not be overwritten. An optional user-installed alias is separate from the plugin-local binary.

### 4.1 Three runtime operations

All flags below are the proposed public contract to implement, not commands already available:

```sh
jacu prepare --repo PATH --request-file PATH --format json
jacu prepare --repo PATH --request-file PATH --audit --format json
jacu prepare --repo PATH --request-file PATH --contract-file PATH --session ID --format json
jacu verify --repo PATH --session ID --checkpoint iteration --format json
jacu verify --repo PATH --session ID --checkpoint delivery --format json
jacu report --repo PATH --session ID --format json
jacu report --repo PATH --session ID --format markdown
```

`prepare` can first inventory evidence, then accept the same agent's compact contract for that session. It validates IDs, references and policy constraints; it does not pretend Rust can infer arbitrary business requirements. If the task is already structured, preparation can happen in one invocation. Do not force a two-call ritual when unnecessary.

Reserve internal adapter paths under these operations, such as `verify --hook HOST` and `report --capabilities`; they are not additional user workflows. A delivery stop hook must use evaluation-only semantics, not launch a long suite inside the hook. Normal `verify` executes outstanding checks.

Unknown required fields, incompatible schemas and truncated essential output are errors. A missing optional evaluator is not a global error. Never run an unknown command supplied by a model merely because it appears in a JSON field.

### 4.2 Local storage

Use bounded JSON/JSONL, not a database service. Prefer a plugin/user data directory keyed by canonical repository and session identity; link to receipt paths from outputs. Do not create tracked status files on every edit. An optional `.jacu-fast.json` policy is the only project configuration file; defaults must work without it.

Writes must be atomic, concurrent appends protected and corrupt/incomplete records recoverable as unknown. Store bounded diagnostics separately. Record summaries and hashes, not full repository copies or raw conversations. Exclude local records from Git; document retention and user-controlled removal.

Schema and binary versions are explicit. Pin a session to a compatible version; do not silently migrate an active session during an update.

## 5. Outcome contract and no-shortcut completion

Bind the contract to the original request or specification and its content hash. Every requested outcome has an ID, source locator, observable result, implementation state, required evidence and delivery state. One useful check may support several requirements.

The current agent cross-checks the mapping against the original request before implementation. An optional semantic assessment can flag omissions. Neither a generated checklist nor a high model score proves the original request was captured perfectly; the source remains available for audit.

Implementation states: `pending`, `present`, `not_applicable`. Evidence is a separate dimension. Delivery states: `pending`, `in_target`, `not_applicable`. An inapplicable requirement requires a source-backed reason; it is not a convenient way to remove scope.

A later user instruction can amend scope. Record its provenance and show the amendment; the agent cannot approve its own reduction. No runtime confirmation flow is introduced. When the original task is impossible, keep the unmet outcome and report it rather than asking to waive it.

Inspect relevant changes for placeholders, hard-coded success, skipped negative cases, swallowed errors, weakened assertions and mock replacements for required real integration. These are investigation candidates, not keyword convictions: a legitimate mock or old TODO is not automatically a defect.

### 5.1 Completion predicate

Within the declared scope, `complete` requires:

1. Every active outcome is present or demonstrably inapplicable.
2. Mandatory evidence is valid for the actual delivered candidate.
3. No known mandatory failure or unknown required evidence remains.
4. Every task-required contribution is reconciled into the agreed target.
5. There is no unacknowledged scope reduction.
6. The report uses that candidate and the same receipts.

This predicate checks the recorded delivery contract, not universal program correctness. An unrestricted local agent can edit local files; hashes alone do not make receipts tamper-proof. Trusted CI recomputes its own obligations rather than accepting agent-writable records as proof.

### 5.2 States and exit results

Use command `outcome` values `ready`, `complete`, `incomplete`, `failed`, `unavailable`, plus an `assessment_complete` value for an audit. Audit completion is not certification that the application is correct.

Check states: `passed`, `failed`, `reused`, `not_applicable`, `deferred`, `unknown`. `deferred` is iteration scheduling only; a delivery-required check remains outstanding. `reused` is reserved for a still-valid successful receipt. A prior failure can be returned without rerunning, but remains `failed`.

Normal exit codes: `0` for command success with its valid ready/complete/audit outcome, `2` for a known check failure, `3` for incomplete/unavailable required evidence, `4` for invalid input/configuration or runtime failure. The JSON outcome is always authoritative alongside the documented exit semantics. Never treat an exit-zero preparation as completed development. Do not report signal/timeout termination as success.

Host hooks have their own response protocol; map the internal result correctly instead of forwarding normal CLI exit codes blindly. A blocked completion attempt returns next actions to the agent, not an approval request.

## 6. Git inventory and integration without lost work

At preparation, inspect only the scoped repository and linked worktrees. Start with metadata: current/base/delivery refs, staged and unstaged state, untracked files, detached heads, stashes, locks, missing paths and candidate unique commits. Include ignored data when evaluating removal safety; do not read secret file contents into model context.

Use machine-readable Git output and robust paths. `git worktree list --porcelain -z` supports unambiguous parsing. GitHub alone cannot see local uncommitted files or unregistered directories. Report inspected roots and omissions. [S09]

Disable external diff/textconv execution for read-only comparisons and avoid executing repository hooks during an audit. Treat revision names and paths as structured arguments; guard option injection and path escapes. Use existing Git rather than rewriting it.

Inspect candidate contents only when potentially relevant. Classify work as `needed`, `already_represented`, `conflicting`, `unrelated`, `active` or `unknown`, with evidence and provenance. JEV can help assess overlap but is not an authority to merge or delete.

The current agent performs permitted integration through existing Git tools. The CLI tracks and checks it; do not write a conflict-resolution engine. Default delivery target is the current task checkout unless the task names another target. Creating the new Jacu Fast repository is authorized by this handoff; runtime tasks do not inherit a blanket mandate to push to `main`.

No blind merge of every branch, `reset --hard`, forced stash drop, force-push or removal of someone else's dirty checkout. Resolve relevant conflicts by code and verification. Preserve unresolved or unrelated work without asking. Unknown work blocks delivery only when it is material to the requested outcome.

Before completion, recheck task-scoped contributions and the combined candidate. Reachability or patch similarity is useful but does not establish that behavior survived later changes. Saving a branch is preservation, not integration. Never silently recreate an implementation that has already been located elsewhere.

## 7. Verification policy: do less unnecessary work

### 7.1 Scheduling

Prepare once. Implement a coherent behavior slice. Choose the next useful check: a known failing test, compiler feedback, a focused integration test, an existing smoke/E2E or a native harness. Do not run `check -> clippy -> all tests -> full review` mechanically after each edit.

Preserve focused TDD when the project uses it. Reduce repeated scope, not the evidence required for completion. A syntax-only change need not trigger a new architecture review. A one-line authorization change may require substantial verification.

Order suitable checks by their observed usefulness, duration and project criticality. A known regression check normally precedes unrelated broad execution. Resource-lock awareness matters; launching multiple builds can make every result slower.

### 7.2 Initial selection, not invented impact analysis

Use existing check catalogs and project instructions; add minimal task-local groups when a trusted project has none. Start with package/target-level structure and explicit applicability rules. Cargo metadata describes packages and targets, not a complete function-to-test dependency graph. [S07]

Unknown impact expands scope or remains an explicit limitation; it is not guessed away by JEV. Changes to shared code, feature configuration, native build inputs, lockfiles, migrations, generated interfaces or the checking policy invalidate relevant assumptions.

Support existing smoke/E2E/native checks through the same generic adapter. Do not require Playwright for a native macOS application. Do not replace the project's runner or force new dependencies merely to use Jacu.

### 7.3 Reuse contract

First implement same-session, unchanged-state deduplication. A receipt identity includes repository/worktree, source content including relevant dirty and untracked inputs, command and argument vector, toolchain, features, policy version, test/fixture inputs and relevant environment identity.

Never use only commit SHA, changed filenames, timestamps or a model's low-risk score. Do not reuse results from another worktree just because branch names match. Secrets may affect a declared environment identity but must not be stored as raw values or weakly hashed low-entropy secrets.

Tests involving changing network services, time, entropy, credentials or stateful databases are not hermetic by default. Disable reuse unless a specific check defines a valid contract. Compilation caches, executed-test receipts and semantic-assessment caches are different things.

Bind results to the observed candidate before and after execution. Reject observed concurrent edits, invalidated snapshots and incomplete capture. Cooperative locks protect Jacu-owned operations but do not prevent arbitrary external writers. High-assurance CI verification uses a clean immutable candidate; do not claim a before/after hash proves the absence of all transient external edits.

Serialize duplicate Jacu-managed work on a compatible resource key. Do not kill unrelated Cargo builds. Do not allow stale results to approve newer code.

### 7.4 Rust diagnostics

Measure cold/warm builds, setup, execution and resource waits separately. Cargo test filters narrow the tests the executable runs; they do not necessarily avoid compiling/linking that executable. `--no-run` and `--timings` help characterize build cost. `cargo check` is useful but does not execute behavior. [S06, S08]

Only after measurement, experiment with package/target selection, feature stability, test grouping, nextest, incremental settings, profile/debuginfo choices or a compile cache. Do not automatically use `--all-features`, remove serialization or assume incremental and a cache wrapper produce additive benefits. Keep one controlled change per comparison.

The private pilot's approximately ten-minute Cargo test is user-reported, not an instrumented baseline. No promise of a 10-minute-to-10-second speedup is authorized.

### 7.5 CI

Use existing CI, not a new scheduler. Inspect redundant triggers, matrix breadth, repeated setup/build, cache misses, obsolete runs and coverage on every tiny iteration. GitHub Actions provides concurrency control; apply cancellation only to disposable verification, not blindly to deployment or stateful migration jobs. [S10]

Use stable trusted policy for required checks. A PR cannot downgrade its own obligations silently. Local receipts are useful developer evidence, not trusted cross-machine credentials. Keep public-fork jobs away from secrets; never run untrusted candidate code with privileged publication tokens.

## 8. Proportional effort without agent management

The current agent supplies a short structured score, using observed evidence:

| Dimension | 0 | 1 | 2 |
|---|---|---|---|
| Uncertainty | Clear outcome and known approach | Some investigation needed | Cause/acceptance materially unclear |
| Novelty | Reuse established local pattern | Adapt known components | New interface/design/dependency |
| Coupling | Local behavior | Several known components | Cross-boundary or concurrent interactions |

Record consequence separately: `ordinary`, `sensitive`, `critical`, `unknown`. Do not average it away. Initial mapping: total 0–1 -> focused, 2–4 -> standard, 5–6 -> deep; sensitive/critical work imposes an appropriate floor. These are versioned experimental bands, not universal token budgets or calibrated probabilities.

One-line rationale per dimension is enough. No expensive model call just to classify an obvious task; no recurring rescore after each edit. Escalate when meaningful ambiguity, sensitive behavior, integration conflicts or unexplained failures appear. Effort never reduces active outcomes or mandatory evidence.

Record `recommended_effort`, `requested_effort`, `applied_effort`, `application_status`, host/model/version and the reason. Unsupported application is `advisory_only`; unobservable values are null, not assumed applied.

Claude skills document an effort setting, while Codex has host/model-specific configuration. A fixed skill setting is not dynamic task-level control. Use only actual supported task/session controls; do not rewrite global configuration, simulate user commands, spawn another agent or claim to change reasoning already spent. [S02, S05]

The portable saving is smaller context, fewer speculative alternatives, fewer repeated reviews and clear stopping conditions. Native effort adjustment is an enhancement whose incremental benefit must be measured.

## 9. Less code and compact context

Before adding code, locate existing implementations, helpers, checks and branch contributions. Prefer small patches over regenerating whole files. Justify new dependencies and abstractions against outcomes. Do not rewrite unrelated files to satisfy Jacu's preferences.

Keep concise meaningful source identifiers. No `f17` renaming campaign or private programming language. Instead, create a task-scoped context map when helpful:

```text
R03: saved settings survive restart
S17: src/settings/store.rs::save(Settings) -> Result<()>
T04: settings_survive_restart
Action: reuse S17; verify R03 through T04
```

IDs are references in context, not replacement code. Bind them to current definitions, paths and content identity; expand before editing. Do not pretend regex resolves every language symbol. Use available metadata/search without building a general language server.

Keep only the current delta and relevant diagnostics in the hot context. Raw logs remain accessible and bounded; summaries must not erase failures or acceptance criteria. Aider's repository-map approach is useful background, not a dependency requirement. [S15]

Any compaction experiment must count expansion, dictionary, cache, tokenization and correction costs. Fewer characters do not automatically mean proportionally less latency. [S16]

## 10. Optional JEV integration

JEV remains a semantic accelerator, not the product's engine. Use an already-configured compatible local command or a tool already exposed by the host. No provider login, key storage, setup wizard, direct SDK proliferation or silent fallback to a paid model.

Where the host alone has a JEV tool, the skill forwards a bounded request and imports a response bound to request/evidence identity. A subprocess does not magically inherit host MCP tools. Mark host-imported evidence separately from CLI-collected evidence.

Determine the supported adapter explicitly: several community programs have used the `jev` name with different interfaces. Never assume that executable name establishes compatibility.

Use narrow structured questions: potential plan omissions, unnecessary scope, overlap with existing work or contradictions between claims and observations. Batch independent questions over the same small state. TypeSafe documents this separation of code-owned control and semantic decisions. [S11]

Validate schema and question IDs, finite/range-constrained numbers, option membership and evidence binding. For Noul, the documented result is `noul`; Choice has option probabilities and may include `confidence`. Do not invent a free-form explanation as output from a typed-only answer. Missing/unsupported answers are unavailable, not favorable votes. [S12, S13]

Record requested/served model, input identity, question version, latency and known usage. Probabilities are not a probability that the entire application is correct. No model score can clear a failed deterministic check, grant permission, erase scope or silently suppress mandatory security evidence.

If JEV is missing, slow, incompatible, rate-limited or disallowed by egress policy, stop attempting that optional enhancement and continue independently executable work. A remote service may receive data even if its client is configured locally. Do not send secrets, proprietary content prohibited by policy, or entire repositories unnecessarily.

No mandatory JEV check is part of the default release closure predicate. If a user explicitly requests a semantic experiment, its unavailable outcome is reported as that experiment's limitation, not a general CLI failure.

## 11. Non-interactive execution and recovery

Run structured program/argument arrays, without interpolating repository or model content into shell commands. Do not allocate a terminal. Use null stdin or a bounded machine payload and close it. Drain stdout/stderr concurrently, bound retained bytes and enforce configured command deadlines. [S14]

A deadline must suit the actual check; do not kill a legitimate ten-minute suite because a fast-feedback target was 30 seconds. Use observed durations/configured runner limits. A timeout remains a failure or unknown, never a pass.

Disable known pagers/editors/auth prompts per adapter. Git terminal-prompt suppression alone does not cover arbitrary askpass, SSH or credential helpers; test supported non-interactive transports. Do not pipe `yes`, run `sudo`, launch a browser login or install missing toolchains during recovery. A repository script can be arbitrary code; Jacu cannot guarantee an unknown GUI-spawning process will obey a no-prompt environment. Use documented adapters, host isolation and deadlines, and report that boundary.

Recover only with a relevant change, a new hypothesis or a justified transient retry. Preserve first failure and all counted retries. Repeated identical no-progress attempts return the existing problem rather than spending indefinitely. A genuine terminal result must be allowed by the stop hook; it is not a human approval checkpoint.

Operational states must include terminal outcomes, not an endless agent loop. External denial, unavailable required hardware and user cancellation are reported immediately and accurately. Never alter host permission modes or claim all external interruptions can be eliminated.

## 12. Hygiene and worktree cleanup

0.1 provides inventory and recommendations, automatically pruning only its own expired diagnostic records under its private data directory. Do not run `cargo clean` after every task: that can trade disk space for slower future development.

Distinguish build outputs, useful caches and actual work. Record logical versus allocated size when available and avoid double-counting symlinks/shared files. Git worktree administrative pruning is not the same as deleting an existing checkout. [S09]

S7 adds narrow automatic cleanup without an approval dialog. Eligible targets must be Jacu-owned, regenerable, inactive, inside a fixed allowed root and still identical to the inspected resource immediately before removal. Prefer TTL/quota removal of Jacu logs, traces and task-owned disposable artifacts before shared compile caches.

For task-owned worktrees, require reconciled contributions, no unique required commits, no tracked/untracked/important ignored data, no lock or active process, and a tested safe Git removal path. If exclusion of external writers cannot be established, preserve the worktree. Never use age, a merged PR label or a JEV score as sole evidence to delete.

Everything ambiguous or unrelated is automatically preserved. No interactive `clean apply` authorization step is required; eligibility comes from the invocation scope and trusted ownership policy, not a model-generated waiver. A preflight cleanup plan can exist as a machine record, not a user checkpoint.

Measure bytes actually recovered together with the cost of the next build. A cleanup incident is a correctness failure even if a disk-space metric improves.

## 13. Plugin packaging and installation

### 13.1 One skill source, thin host adapters

Maintain one canonical `skills/jacu-fast/SKILL.md` with short references. Package the same body as Claude's `skills/run/SKILL.md` (name `run`) and the portable `skills/jacu-fast/SKILL.md` (name `jacu-fast`). Only host-specific frontmatter, path resolution and hook envelopes differ.

Use a portable root `plugin.json` plus the Claude manifest in `.claude-plugin/plugin.json` where needed. Current OpenAI documentation favors the portable root manifest; `.codex-plugin/plugin.json` is a compatibility fallback, not a mandatory second canonical definition. Add host-specific fields only after validating them. [S04]

Do not ship `agents/`, `context: fork`, MCP connections, model overrides, permission bypasses or empty optional integrations. Missing host hooks mean explicitly advisory enforcement, not fake enforcement.

### 13.2 Prebuilt distribution

Build native executables in CI, with a versioned manifest binding binary hashes, skill version, protocol version and source commit. Include executable permissions and necessary platform metadata. Verify checksums and publish build provenance where supported. Do not tell users to compile Rust or run Cargo for first use.

For Claude, publish a self-contained release ZIP through a marketplace archive source with its SHA-256. This avoids a source-only plugin that lacks its executable. Claude documents archive sources and plugin `bin/` support; distribution channels have different restrictions. [S01, S17]

A practical initial archive can contain only the tested target binaries in explicit platform subdirectories. Resolve the current target once using trusted host/OS data and a fixed mapping. Hook command glue may select from this finite map; all product operations then call Rust. No first-task download, install script, interpreter service or platform guess. Unsupported platform -> unavailable with a truthful result.

For a local Codex-compatible host, validate its actual package distribution path and access to bundled binaries. Do not assume Claude archive-source support is portable. A generated local package or immutable Git-backed distribution is acceptable if it is documented and tested end to end; no hand-configured credentials or model connections. Public directory submission is distinct from creating a GitHub repository and may require external review. [S04]

Keep source `main` free of committed large release binaries. Marketplace metadata must reference an existing immutable release and actual checksum before being advertised. No fake URLs, zero checksums or placeholder installation badge in a release.

Pin active sessions; updates occur through the host's supported mechanism, not a Jacu self-updater. Uninstall removes plugin resources only, preserving source and unrelated work. Clean up only Jacu-owned state according to its documented retention policy.

### 13.3 Hooks: tiny and active-task-scoped

Use native command hooks where supported, not model-based hooks. On edit/tool events, mark receipts as potentially stale and return quickly; do not run tests or call JEV. Hook events are hints, not a complete mutation journal: revalidate before reuse.

At completion, evaluate the current receipt/contract state. If incomplete and recoverable, return specific next actions to the existing agent. If complete, inactive, user-cancelled or terminally blocked with a truthful report, do not force continuation. Handle recursion/no-progress explicitly.

Use a fast session identity keyed to the host session and canonical repo. Do not scan a repository or affect unrelated conversations on every global hook. Avoid recursively reacting to Jacu's own records.

Test commands in paths containing spaces, non-ASCII characters and shell metacharacters. Only fixed wrapper text may pass through a shell; task text, paths from Git and model-generated strings never become shell fragments.

Codex hook trust and availability are host-controlled; installing a plugin is not automatically trusting executable hooks. In cloud interfaces, packaging does not deploy local scripts or grant laptop access. Document these facts instead of adding a bypass. [S04]

### 13.4 Public material

README: value proposition, exact supported install path, no-prompts behavior, optional JEV, truthful completion semantics, supported hosts/platforms and limitations. Include LICENSE, CONTRIBUTING, SECURITY, a short CHANGELOG and this plan or its implementation contract.

Public examples must be synthetic. Do not copy raw private pilot code, source excerpts, local paths, screenshots, receipts, issues, access-bearing URLs or conversations into GitHub. Sanitized aggregate results require appropriate permission. The predecessor remains private/read-only unless independently authorized otherwise.

## 14. Repository creation and publication

Create the new repository using the executor's existing GitHub authorization. Check exact identity first. If it already exists, inspect ownership, visibility and history; never overwrite or republish an unrelated/private repository. A missing repository is not the same as a failed authentication request.

For a confirmed absent repository, GitHub CLI supports explicit non-interactive creation. [S18]

```sh
gh repo create jacu-dev/jacu-fast \
  --public \
  --description "A lightweight Rust plugin for faster, complete AI-assisted development." \
  --license MIT \
  --gitignore Rust \
  --add-readme \
  --clone
```

This is an executor instruction, not a claim that this handoff created the repository. Reuse installed credentials; never start `gh auth login` or expose tokens. Explicitly set/verify `main` as default after initialization; repository creation can inherit account defaults. Do not force-overwrite an existing working directory.

Publish short implementation slices with honest commit messages. Once checks exist, protect the delivery path using required CI checks where available; do not add a manual review ritual just to replace the forbidden Jacu approval flow. Preserve organization policies if they already require approvals.

Use pinned/action-reviewed CI dependencies, least-privilege tokens and separate untrusted PR verification from release permissions. Build/test before release publication. A public GitHub repository is not automatic publication in a host's plugin directory. Record installed/tested, published and externally pending states separately.

## 15. Repository structure to build

```text
jacu-fast/
  Cargo.toml
  Cargo.lock
  rust-toolchain.toml
  src/
    lib.rs
    main.rs
    contract.rs
    inventory.rs
    policy.rs
    runner.rs
    receipt.rs
    report.rs
  tests/                         # synthetic fixtures and integration tests
  skills/jacu-fast/
    SKILL.md                     # canonical body
    references/runtime.md
  packaging/
    plugin.json                  # portable manifest template
    claude-plugin.json           # small Claude overlay
  .claude-plugin/marketplace.json # only after an installable artifact exists
  .github/workflows/
    ci.yml
    release.yml
  docs/
    IMPLEMENTATION_PLAN.md
    BENCHMARK.md
  examples/policy.json
  README.md
  AGENTS.md
  LICENSE
  CONTRIBUTING.md
  SECURITY.md
  CHANGELOG.md
```

This is a ceiling of conceptual components, not a requirement to create empty files. Merge modules when clearer. Do not add a separate framework to generate docs, run benchmarks or manage plugin manifests. The host packages are generated outputs of one implementation.

## 16. Implementation work packages

Each slice produces executable evidence. Maintain one task status file or issue list, not repeated plans and handoffs. Never mark a slice complete from prose alone.

| Slice | Deliverables | Dependencies | Required exit evidence |
|---|---|---|---|
| S0 | New public repo; MIT; English docs; sanitized fixture policy; toolchain and initial source skeleton. | Existing GitHub access | Exact URL, default branch, license and initial commit verified. |
| S1 | Rust contract/types, bounded local records, prepare/verify/report protocol and runner. | S0 | NI and protocol fixtures pass offline; real exit/result capture. |
| S2 | Git preflight, task/source binding, effort rubric, skill-only workflow and audit. | S1 | Lost-work, scope-omission and effort fixtures; audit runs no project code. |
| S3 | Cargo + generic check adapters, iteration/delivery split, focused retries and valid deduplication. | S1–S2 | Zero-test, stale-state, environment and concurrency cases; measured warm/cold diagnostics. |
| S4 | Completion predicate, reconciliation checks, compact context references, optional existing-JEV bridge. | S2–S3 | Incomplete work cannot become complete; missing JEV is usable; no provider login. |
| S5 | Prebuilt plugin packages, one skill, supported command hooks, native effort capability reporting and clean installation. | S1–S4 | Tested install/invoke/uninstall and task-scoped hooks; actual host/platform matrix. |
| S6 | Paired task benchmark, CI optimization report, public sanitized release evidence and 0.1 candidate. | S3–S5 | All release-required cases passed or release remains explicitly incomplete/preview; no fabricated gains. |
| S7 | Narrow ownership-based hygiene application with no prompts. | S2, S6 | Preservation/concurrency fixtures and actual reclaimed bytes plus next-build impact. |

Implement the minimum vertical path early: install/invoke -> task contract -> one real focused check -> truthful report. Do not finish every inventory feature before testing that user experience. A minimal local development package may exercise S5 early; it is not a released feature until the complete slice passes.

Where external access is unavailable, keep implementing independent work and report the blocked item. Do not pretend remote creation, host invocation or publication happened. Do not change acceptance criteria just to close a milestone.

## 17. Acceptance tests

The accompanying `acceptance-cases.json` is a machine-readable checklist to turn into tests. It does not claim those tests have run. Negative cases matter as much as the happy path.

### Non-interactivity and process control

- NI01: open stdin with no input; every operation returns or hits a bounded deadline, never awaits OK.
- NI02: closed stdin and missing optional configuration; return structured output without a wizard.
- NI03: a child asks a terminal question; no inherited human input, no automatic `yes`, bounded completion.
- NI04: large simultaneous stdout/stderr and machine stdin; no unbounded memory or deadlock.
- NI05: absent credentials never trigger Jacu login, editor, pager or credential UI.
- NI06: repeated unchanged failure does not consume endless retries; terminal reporting is possible.
- NI07: external host denial remains external; no permission-mode rewrite.

### Verification and receipts

- V01: no default full-suite invocation on every edit.
- V02: a filter selects zero tests; it cannot satisfy a behavioral obligation.
- V03: interrupted, timed-out or truncated execution cannot pass.
- V04: identical reusable state uses the existing successful receipt exactly once.
- V05: relevant source, features, toolchain, fixtures, policy or environment change invalidates reuse.
- V06: dirty/untracked input change is not hidden by unchanged commit SHA.
- V07: changing external state disables ordinary reuse unless explicitly modeled.
- V08: simultaneous Jacu requests for the same resource avoid duplicate work; no unrelated process is killed.
- V09: stale/concurrently edited candidate cannot approve a newer delivery.
- V10: delivery only executes missing/invalidated obligations; no mandatory second full run.
- V11: existing smoke/E2E/native output is captured through the same protocol.

### Completion and existing work

- C01: "done" text without implementation/evidence returns incomplete.
- C02: one omitted original outcome remains visible and outstanding.
- C03: self-approved scope reduction or weakened required assertion cannot satisfy closure.
- C04: implemented-but-unverified and verified-but-unintegrated are not complete.
- C05: a real external blocker returns truthful terminal incomplete without a question.
- C06: a valid delivery report references the actual target and all current obligations.
- G01: task implementation in another worktree is found before duplicate implementation.
- G02: unrelated dirty/active/locked work is preserved and does not block independent delivery.
- G03: a reachable historical commit later reverted is not evidence of current behavior.
- G04: combined candidate is checked after permitted integration.
- G05: missing/inaccessible worktree and stale remote refs are reported, not silently assumed safe.
- G06: spaces, non-ASCII, newline-bearing filenames, symlinks and option-like refs are handled safely.

### Effort, semantic assessment and context

- E01: mechanical task maps to focused guidance without a costly extra classifier call.
- E02: consequence floor survives a low line count or low difficulty sum.
- E03: unsupported/unobservable effort is advisory/null, never falsely applied.
- E04: meaningful new evidence can escalate; no per-edit oscillation or subagent spawn.
- SEM01: no JEV available -> deterministic path works, without any paid fallback.
- SEM02: malformed/unknown/missing/non-finite semantic answers cannot approve anything.
- SEM03: no semantic score overrides a required failure or grants destructive authority.
- SEM04: host-imported assessment carries matching request and evidence identity.
- CT01: stale/ambiguous context IDs are rejected or regenerated.
- CT02: meaningful source identifiers and project conventions remain intact.

### Distribution, public boundaries and hygiene

- P01: clean-machine install discovers the entry skill and executes the correct prebuilt binary.
- P02: missing/corrupt/wrong-platform binary never falls back to compiling or fabricating success.
- P03: inactive hooks do no repository work; completion hook performs no long test run or model call.
- P04: stop-hook recursion, interruption and terminal incomplete are bounded.
- P05: active-session versions remain pinned; uninstall preserves project and unrelated data.
- P06: documented host syntax/capabilities match actually tested behavior.
- P07: no private fixtures, credentials, proprietary excerpts or raw pilot receipts are published.
- H01: inventory does not remove targets, worktrees or shared caches.
- H02: later cleanup removes only owned inactive regenerable resources within the allowed root.
- H03: changed identities, active resources, unique commits and ignored/untracked data are preserved.
- H04: disk reporting does not double-count shared paths; next-build cost is measured.
- M01: unknown usage/cost is null rather than zero; counters are not fabricated.
- M02: failed, incomplete and retried tasks remain in the benchmark denominator.
- M03: deferred work and final full checks remain in total cost; skipping is not passing.

## 18. Measuring a large, defensible gain

### 18.1 Comparisons

Run representative tasks under the current workflow, skill-only, skill + Rust core, core + native effort where confirmed, and optional JEV. A small paired corpus with disclosed limits is better than fabricated precision. Keep task families, model/version and cold/warm conditions comparable; vary order to reduce cache/order bias.

The private pilot is for local authorized investigation. Public tests must use synthetic/open examples. Do not assume its native UI is a browser or that its roughly ten-minute Cargo run was all assertion execution.

### 18.2 Primary metrics

Time to complete verified integrated delivery; time to first useful failure; total verification wall time and runner-minutes; compile/setup/run/wait components; duplicate runs avoided; valid reuse; repair cycles; user correction turns; task-related code recovered; code/dependency churn; requested/applied effort; known LLM/JEV usage; omitted outcomes and defects discovered only at delivery or later.

Report p50 and p95 with sample counts where meaningful. No single universal quality score. Do not infer savings from an unobserved baseline or extrapolate a short task to an entire month. LOC is secondary and includes generated/configuration maintenance rather than rewarding code hiding.

### 18.3 Ambitious targets, not claims

Initial experiment targets: at least 60% less median local verification wait on eligible iterative tasks; at least 40% less total CI verification cost where the relevant optimization is applied; a material decrease in accepted-delivery time after including all overhead; zero Jacu approval interruptions; zero known critical escapes or data-loss incidents in the evaluated corpus.

Thirty-second feedback for common warm iterations is a stretch target, not a default timeout or SLA. No universal target can ignore builds, native dependencies or required tests. Report achieved values even when targets are missed.

A report of zero escapes in a small corpus is not proof of zero risk. Tune on one set, evaluate on held-out task families/changes and state sample limits. Keep the reference checks/known defects outside the candidate agent's answer context.

### 18.4 Experiments retained from the discussion

| ID | Trial | Promotion condition |
|---|---|---|
| X1 | Existing Cargo/CI improvements, adjusted scope, replay and caching without JEV. | Net time saved under valid equivalent evidence conditions. |
| X2 | Explore a new UI path with observed actions and optional JEV; replay without AI. | Independent assertions catch regressions; replay and repairs outperform repeated exploration. |
| X3a | JEV-first local iteration with fewer unit tests, using selected executable checks. | Meets predeclared detection and omission limits on held-out changes; total cost is lower. |
| X3b | JEV-only laboratory judgment of plan/diff without executing candidate tests. | Measured against independent reference behavior; no production correctness claim merely from a score. |
| X4 | Compact task maps/aliases in context only. | Net delivery time/tokens improve after expansion and rework; no source obfuscation. |

For X2, a changed locator may be repaired; a changed expected amount, permission or persistence result may not be weakened to make a run green. Preserve first failures. Observed UI actions must be validated by the executor; do not execute generated arbitrary JS or shell.

For X3, preserve the reference suite while evaluating. Count its cost and distinguish research cost from proposed runtime cost. Reject or restrict an optimization that misses important defects rather than hiding a negative result. The product succeeds if conventional engineering provides the gain and JEV is unnecessary.

## 19. Research retained, with limits

These are leads collected in the earlier research, not Jacu measurements or independent reproductions. The executor must reread the current source before quoting numerical claims in a public release. This final plan deliberately does not carry forward unverified speedup numbers as promises.

| Research lead | Transferable idea | Evidence limit |
|---|---|---|
| gemanor/jev-code-review-benchmark | Typed, bounded code-review questions. | Constructed review cases do not establish full-PR or security correctness. |
| browser-use/jev-ultrafast | Reduce observation/action round trips. | Small same-task comparisons; optimizing a JEV workflow is not measuring JEV versus no JEV. |
| openqa-cn/jev-browser | Explore then generate deterministic replay. | Implementation precedent, not broad regression benchmark. |
| Ying-Kai-Liao/jev-browser | Closed action choices with independent completion checks. | Small author-reported benchmark with remaining failures. |
| Callstack Jevil and replay articles | Delegate bounded navigation; replay stable flows. | Demo/limited repeated paths; replay gains are not all attributable to JEV. |
| huaaudio/jevsim | Avoid returning to the main LLM at every interaction. | Narrow native navigation trial. |
| Meta predictive test selection | History/impact-aware execution can remove substantial redundant work. | Different industrial system; not evidence that a generic JEV question reproduces it. |
| clduab11/jev-test | Evaluate end-to-end behavior, including negative results from adding a judge. | Non-coding benchmark; useful warning, not a coding result. |

Research URLs are in the source register. Do not include entire third-party documents or private historical handoffs in the runtime plugin.

## 20. Final executor delivery contract

Deliver the actual repository URL, source commit, completed slice IDs, runtime tests executed, host/platform installation results, real benchmark rows, artifact checksums and all remaining limitations. Separate source implementation, integration validation, release publication and external marketplace approval.

Do not state "100% complete" if a required milestone, test, host install or requested publication is pending. Also do not stop at planning or asking whether to proceed: implement independent slices within available capabilities, preserving state and returning an honest terminal result when genuinely blocked.

The end-user summary must come from receipts and outcome state, not a model's optimistic narrative. The product's rule is:

**Do less unnecessary work, never less than was agreed.**

## 21. Sources and verification status

Primary documentation rechecked on September 24, 2026; URLs may evolve and implementation must validate the installed host version. Product choices, module boundaries, rubrics and targets above are Jacu design decisions, not claims made by these sources.

- **S01 — Claude plugins and namespacing:** https://code.claude.com/docs/en/plugins
- **S02 — Claude skills and effort:** https://code.claude.com/docs/en/skills
- **S03 — Claude plugin components, executables and hooks:** https://code.claude.com/docs/en/plugins-reference
- **S04 — OpenAI plugin packaging, portable manifests and hook limitations:** https://developers.openai.com/plugins/build/plugins
- **S05 — Codex configuration reference:** https://developers.openai.com/codex/config-reference
- **S06 — Cargo test:** https://doc.rust-lang.org/cargo/commands/cargo-test.html
- **S07 — Cargo metadata:** https://doc.rust-lang.org/cargo/commands/cargo-metadata.html
- **S08 — Cargo check:** https://doc.rust-lang.org/cargo/commands/cargo-check.html
- **S09 — Git worktrees:** https://git-scm.com/docs/git-worktree
- **S10 — GitHub Actions concurrency:** https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency
- **S11 — TypeSafe composition guidance, also queried through Context7:** https://docs.typesafe.ai/concepts/how-to-build-with-system-one
- **S12 — TypeSafe Noul:** https://docs.typesafe.ai/primitives/noul
- **S13 — TypeSafe confidence:** https://docs.typesafe.ai/confidence
- **S14 — Rust subprocess standard streams:** https://doc.rust-lang.org/std/process/struct.Stdio.html
- **S15 — Aider repository maps:** https://aider.chat/docs/repomap.html
- **S16 — OpenAI latency optimization:** https://developers.openai.com/api/docs/guides/latency-optimization
- **S17 — Claude marketplace source types and archive checksums:** https://code.claude.com/docs/en/plugin-marketplaces
- **S18 — Non-interactive repository creation:** https://cli.github.com/manual/gh_repo_create
- **S19 — OpenAI skill authoring:** https://developers.openai.com/plugins/build/skills
- **S20 — Artifact provenance:** https://docs.github.com/actions/security-for-github-actions/using-artifact-attestations/using-artifact-attestations-to-establish-provenance-for-builds

Historical research leads, carried forward from the supplied research draft and not reproduced here:

- https://github.com/gemanor/jev-code-review-benchmark
- https://github.com/browser-use/jev-ultrafast
- https://github.com/openqa-cn/jev-browser
- https://github.com/Ying-Kai-Liao/jev-browser
- https://www.callstack.com/blog/exploring-jev-for-mobile-qa-with-agent-device
- https://www.callstack.com/blog/3-minutes-with-an-agent-9-seconds-on-replay
- https://github.com/huaaudio/jevsim
- https://engineering.fb.com/2018/11/21/developer-tools/predictive-test-selection/
- https://arxiv.org/abs/1810.05286
- https://github.com/clduab11/jev-test

**Handoff status:** this session produced the specification, seed assets and acceptance checklist. It did not create a repository, compile Jacu, install a plugin, inspect local worktrees, run the private pilot, reproduce benchmarks or publish a release. Those are explicit executor deliverables.
