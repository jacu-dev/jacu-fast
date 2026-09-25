mod inventory;
mod runner;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const SCHEMA_VERSION: u32 = 1;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Checkpoint {
    Iteration,
    Delivery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Json,
    Markdown,
}

pub struct Output {
    pub exit_code: i32,
    pub body: String,
}

pub struct PrepareRequest {
    pub repo: PathBuf,
    pub request_file: PathBuf,
    pub audit: bool,
    pub contract_file: Option<PathBuf>,
    pub session: Option<String>,
}

pub struct VerifyRequest {
    pub repo: PathBuf,
    pub session: String,
    pub checkpoint: Checkpoint,
}

pub struct ReportRequest {
    pub repo: PathBuf,
    pub session: String,
    pub format: ReportFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Session {
    schema_version: u32,
    binary_version: String,
    session_id: String,
    repo: String,
    repo_key: String,
    audit: bool,
    request_sha256: String,
    request_bytes: u64,
    contract: Option<Contract>,
    policy_sha256: String,
    receipts: Vec<Receipt>,
    candidate_hash: String,
    head: Option<String>,
    dirty: bool,
    #[serde(default)]
    bound_minimums: BTreeMap<String, u32>,
    #[serde(default)]
    delivery_target: Option<DeliveryTarget>,
    #[serde(default)]
    bound_checks: BTreeMap<String, String>,
    #[serde(default)]
    bound_required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Contract {
    schema_version: u32,
    outcomes: Vec<OutcomeSpec>,
    effort: EffortSpec,
    #[serde(default)]
    amendments: Vec<Amendment>,
    #[serde(default)]
    context: Vec<ContextRef>,
    #[serde(default)]
    delivery_target: Option<DeliveryTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct DeliveryTarget {
    path: String,
    #[serde(default)]
    branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OutcomeSpec {
    id: String,
    statement: String,
    implementation: String,
    delivery: String,
    #[serde(default)]
    evidence: Vec<String>,
    #[serde(default)]
    inapplicable_reason: Option<String>,
    #[serde(default)]
    source_quote: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EffortSpec {
    uncertainty: u8,
    novelty: u8,
    coupling: u8,
    consequence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Amendment {
    id: String,
    #[serde(default)]
    removes: Vec<String>,
    #[serde(default)]
    weakens: Vec<String>,
    reason: String,
    source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ContextRef {
    id: String,
    path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Receipt {
    check_id: String,
    state: String,
    exit_code: Option<i32>,
    candidate_hash: String,
    argv: Vec<String>,
    cwd: String,
    policy_sha256: String,
    duration_ms: u128,
    stdout_sha256: String,
    stderr_sha256: String,
    stdout_excerpt: String,
    stderr_excerpt: String,
    truncated: bool,
    detail: String,
    #[serde(default)]
    external: bool,
    #[serde(default)]
    env_sha256: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    request_sha256: String,
    #[serde(default)]
    program_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyFile {
    #[serde(default = "schema_one")]
    schema_version: u32,
    #[serde(default)]
    checks: Vec<CheckSpec>,
    #[serde(default)]
    delivery: DeliverySpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckSpec {
    id: String,
    #[serde(default = "external_kind")]
    kind: String,
    program: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default = "dot_cwd")]
    cwd: String,
    timeout_seconds: u64,
    #[serde(default)]
    minimum_tests: u32,
    #[serde(default = "default_reuse")]
    reuse: String,
    #[serde(default)]
    external_state: bool,
    #[serde(default)]
    environment: Vec<String>,
    #[serde(default)]
    inputs: Vec<String>,
    #[serde(default = "score_default")]
    minimum_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct DeliverySpec {
    #[serde(default)]
    required_check_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CandidateView {
    repo: String,
    head: Option<String>,
    dirty: bool,
    hash: String,
    git: bool,
}

#[derive(Debug, Clone, Serialize)]
struct InventoryView {
    git: bool,
    toplevel: String,
    head: Option<String>,
    branch: Option<String>,
    dirty: bool,
    worktrees: Vec<String>,
    notes: Vec<String>,
    disk_logical_bytes: u64,
    disk_allocated_bytes: u64,
    disk_scope: String,
}

#[derive(Debug, Clone, Serialize)]
struct EvidenceView {
    id: String,
    state: String,
    detail: String,
    excerpt: String,
}

#[derive(Debug, Clone, Serialize)]
struct EffortView {
    uncertainty: u8,
    novelty: u8,
    coupling: u8,
    consequence: String,
    recommended_effort: String,
    applied_effort: Option<String>,
    application_status: String,
}

#[derive(Debug, Serialize)]
struct Envelope {
    schema_version: u32,
    binary_version: &'static str,
    command: &'static str,
    session_id: Option<String>,
    request_sha256: String,
    outcome: String,
    terminal: bool,
    exit_code: i32,
    candidate: CandidateView,
    evidence: Vec<EvidenceView>,
    pending: Vec<String>,
    next_actions: Vec<String>,
    timings_ms: BTreeMap<String, u128>,
    error: Option<String>,
    effort: Option<EffortView>,
    inventory: Option<InventoryView>,
    checks: Vec<EvidenceView>,
    outcomes: Vec<OutcomeView>,
    usage: UsageView,
    counts_in_denominator: bool,
    delivery_target: Option<DeliveryTarget>,
    assurance: Value,
    policy_source: String,
    discovered_checks: Vec<CheckSpec>,
}

#[derive(Debug, Clone, Serialize)]
struct OutcomeView {
    id: String,
    implementation: String,
    delivery: String,
    evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct UsageView {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cost: Option<f64>,
}

struct RepoFacts {
    canonical: PathBuf,
    git: bool,
    head: Option<String>,
    branch: Option<String>,
    dirty: bool,
    worktrees: Vec<String>,
    notes: Vec<String>,
    candidate_hash: String,
    disk_logical_bytes: u64,
    disk_allocated_bytes: u64,
}

struct PolicyState {
    raw_sha256: String,
    checks: Vec<CheckSpec>,
    required: Vec<String>,
    source: String,
}

struct Lock {
    file: fs::File,
}
impl Drop for Lock {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            unsafe {
                libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
            }
        }
    }
}
fn score_default() -> f64 {
    0.8
}
fn effective_minimum(c: &CheckSpec) -> u32 {
    if c.kind.contains("test") {
        c.minimum_tests.max(1)
    } else {
        c.minimum_tests
    }
}
fn within(root: &Path, path: &str) -> Result<PathBuf, String> {
    if Path::new(path).is_absolute()
        || Path::new(path)
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("input path must be relative and remain within the repository".into());
    }
    resolve_cwd(root, path)
}
fn snapshot_inputs(root: &Path) -> Result<Vec<String>, String> {
    let path = root.join(".jacu-fast.json");
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let policy: PolicyFile =
        serde_json::from_slice(&read_limited(&path, 262144)?).map_err(|e| e.to_string())?;
    Ok(policy.checks.into_iter().flat_map(|c| c.inputs).collect())
}
fn read_limited(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("input exceeds the size limit".into());
    }
    Ok(bytes)
}

fn schema_one() -> u32 {
    1
}
fn external_kind() -> String {
    "external".to_string()
}
fn dot_cwd() -> String {
    ".".to_string()
}
fn default_reuse() -> String {
    "same-session-declared-inputs".to_string()
}

pub fn invalid_output(command: &'static str, message: &str) -> Output {
    let envelope = Envelope {
        schema_version: SCHEMA_VERSION,
        binary_version: VERSION,
        command,
        session_id: None,
        request_sha256: String::new(),
        outcome: "unavailable".into(),
        terminal: true,
        exit_code: 4,
        candidate: empty_candidate(),
        evidence: Vec::new(),
        pending: vec![message.to_string()],
        next_actions: Vec::new(),
        timings_ms: BTreeMap::new(),
        error: Some(message.to_string()),
        effort: None,
        inventory: None,
        checks: Vec::new(),
        outcomes: Vec::new(),
        usage: UsageView::unknown(),
        counts_in_denominator: false,
        delivery_target: None,
        assurance: serde_json::json!({"verified":false}),
        policy_source: "unavailable".into(),
        discovered_checks: Vec::new(),
    };
    Output {
        exit_code: 4,
        body: serde_json::to_string(&envelope).unwrap_or_else(|_| "{\"exit_code\":4}".into()),
    }
}

pub fn prepare(request: PrepareRequest) -> Output {
    let started = Instant::now();
    match prepare_inner(request, started) {
        Ok(output) => output,
        Err(message) => invalid_output("prepare", &message),
    }
}

pub fn verify(request: VerifyRequest) -> Output {
    verify_retry(request, false)
}
pub fn verify_retry(request: VerifyRequest, retry: bool) -> Output {
    let started = Instant::now();
    match verify_inner(request, started, retry) {
        Ok(output) => output,
        Err(message) => invalid_output("verify", &message),
    }
}

pub fn report(request: ReportRequest) -> Output {
    let started = Instant::now();
    match report_inner(request, started) {
        Ok(output) => output,
        Err(message) => invalid_output("report", &message),
    }
}

fn prepare_inner(request: PrepareRequest, started: Instant) -> Result<Output, String> {
    let facts = inspect_repo(&request.repo)?;
    let request_bytes = read_limited(&request.request_file, 131072)?;
    let request_text = std::str::from_utf8(&request_bytes).map_err(|_| "request must be UTF-8")?;
    if request_bytes.is_empty() {
        return Err("request file is empty".into());
    }
    let request_sha256 = sha256_hex(&request_bytes);
    let policy = load_policy(&facts.canonical)?;
    let session_id = match request.session.as_deref() {
        Some(id) => {
            validate_session_id(id)?;
            id.to_string()
        }
        None => new_session_id(&request_sha256),
    };
    let _lock = lock_session(&facts_key(&facts), &session_id)?;
    let existing = load_session(&facts, &session_id)?;
    if request.session.is_some() && existing.is_none() {
        return Err("unknown session".into());
    }
    let mut session = existing.unwrap_or(Session {
        schema_version: SCHEMA_VERSION,
        binary_version: VERSION.to_string(),
        session_id: session_id.clone(),
        repo: facts.canonical.to_string_lossy().into_owned(),
        repo_key: facts_key(&facts),
        audit: request.audit,
        request_sha256: request_sha256.clone(),
        request_bytes: request_bytes.len() as u64,
        contract: None,
        policy_sha256: policy.raw_sha256.clone(),
        receipts: Vec::new(),
        candidate_hash: facts.candidate_hash.clone(),
        head: facts.head.clone(),
        dirty: facts.dirty,
        bound_minimums: BTreeMap::new(),
        delivery_target: Some(DeliveryTarget {
            path: facts.canonical.to_string_lossy().into_owned(),
            branch: facts.branch.clone(),
        }),
        bound_checks: BTreeMap::new(),
        bound_required: Vec::new(),
    });
    if session.schema_version != SCHEMA_VERSION {
        return Err("session schema is not compatible with this binary".into());
    }
    if session.repo != facts.canonical.to_string_lossy() {
        return Err("session belongs to a different repository".into());
    }
    if session.binary_version != VERSION {
        return Err(format!(
            "session is pinned to jacu {}; this binary is {VERSION}",
            session.binary_version
        ));
    }
    if session.request_sha256 != request_sha256 || session.audit != request.audit {
        return Err("request and audit mode are immutable; preserve this task and create a separate task for a changed request".into());
    }
    session.audit = request.audit;
    session.request_sha256 = request_sha256;
    session.request_bytes = request_bytes.len() as u64;
    session.policy_sha256 = policy.raw_sha256.clone();
    session.candidate_hash = facts.candidate_hash.clone();
    session.head = facts.head.clone();
    session.dirty = facts.dirty;
    session.binary_version = VERSION.to_string();
    if let Some(path) = request.contract_file {
        let next = read_contract(&path)?;
        if let Some(target) = &next.delivery_target {
            let path = fs::canonicalize(if Path::new(&target.path).is_absolute() {
                PathBuf::from(&target.path)
            } else {
                facts.canonical.join(&target.path)
            })
            .map_err(|e| e.to_string())?;
            if path != facts.canonical || target.branch != facts.branch {
                return Err("prepare --repo must name the actual delivery target and its checked-out branch".into());
            }
            let normalized = DeliveryTarget {
                path: path.to_string_lossy().into_owned(),
                branch: target.branch.clone(),
            };
            if session.delivery_target.as_ref() != Some(&normalized) {
                return Err("delivery target is pinned for this task".into());
            }
        }
        for outcome in &next.outcomes {
            if let Some(quote) = &outcome.source_quote {
                if quote.trim().is_empty() || !request_text.contains(quote) {
                    return Err(format!(
                        "source quote for {} is not in the original request",
                        outcome.id
                    ));
                }
            }
        }
        let policy_changes = weakened_pending(&session, &policy);
        if !policy_changes.is_empty() {
            return Err(policy_changes.join("; "));
        }
        if let Some(previous) = &session.contract {
            validate_revision(previous, &next)?;
            let dropped = dropped_outcomes(previous, &next);
            if !dropped.is_empty() {
                return Err(format!(
                    "contract drops outcomes without an amendment: {}",
                    dropped.join(", ")
                ));
            }
        }
        validate_contract(&next, &policy, &facts.canonical)?;
        let minimums = minimum_map(&policy, &next);
        if let Some(problem) = weaken_conflict(&session.bound_minimums, &minimums, &next) {
            return Err(problem);
        }
        session.bound_minimums = minimums;
        session.bound_checks = policy
            .checks
            .iter()
            .map(|c| (c.id.clone(), check_definition(c)))
            .collect();
        session.bound_required = policy.required.clone();
        session.contract = Some(next);
    }
    save_session(&session)?;
    write_diagnostic(&session)?;
    let (outcome, terminal, pending, next_actions) = if request.audit {
        let mut pending = contract_gaps(session.contract.as_ref());
        if policy.checks.is_empty() {
            pending.push("no project policy; audit did not invent checks".into());
        }
        (
            "assessment_complete".to_string(),
            true,
            pending,
            vec!["Audit is read-only. No project command was run.".into()],
        )
    } else if session.contract.is_none() {
        (
            "ready".to_string(),
            false,
            vec!["outcome contract is missing".into()],
            vec![
                "Write a contract file and call prepare again with --contract-file and --session."
                    .into(),
            ],
        )
    } else {
        (
            "ready".to_string(),
            false,
            contract_gaps(session.contract.as_ref()),
            vec!["Implement the pending outcomes, then call verify --checkpoint iteration.".into()],
        )
    };
    Ok(finish(
        "prepare",
        &session,
        &facts,
        &policy,
        &outcome,
        terminal,
        0,
        pending,
        next_actions,
        None,
        started,
    ))
}

fn verify_inner(request: VerifyRequest, started: Instant, retry: bool) -> Result<Output, String> {
    validate_session_id(&request.session)?;
    let facts = inspect_repo(&request.repo)?;
    let _lock = lock_session(&facts_key(&facts), &request.session)?;
    let mut session = load_session(&facts, &request.session)?
        .ok_or_else(|| format!("unknown session {}", request.session))?;
    if session.schema_version != SCHEMA_VERSION {
        return Err("session schema is not compatible with this binary".into());
    }
    if session.binary_version != VERSION {
        return Err(format!(
            "session is pinned to jacu {}; this binary is {VERSION}",
            session.binary_version
        ));
    }
    if session.repo != facts.canonical.to_string_lossy() {
        return Err("session belongs to a different repository".into());
    }
    let policy = load_policy(&facts.canonical)?;
    session.policy_sha256 = policy.raw_sha256.clone();
    session.candidate_hash = facts.candidate_hash.clone();
    session.head = facts.head.clone();
    session.dirty = facts.dirty;
    if session.audit {
        save_session(&session)?;
        return Ok(finish(
            "verify",
            &session,
            &facts,
            &policy,
            "assessment_complete",
            true,
            0,
            vec!["audit sessions do not run project commands".into()],
            vec!["Use report to read the assessment.".into()],
            None,
            started,
        ));
    }
    let mut gate_gaps = weakened_pending(&session, &policy);
    if !target_matches(&session, &facts) {
        gate_gaps.push("observed branch/path does not match the recorded delivery target".into());
    }
    if !gate_gaps.is_empty() {
        return Ok(finish(
            "verify",
            &session,
            &facts,
            &policy,
            "incomplete",
            false,
            3,
            gate_gaps,
            vec!["Restore the bound checks and integrate into the recorded target.".into()],
            None,
            started,
        ));
    }
    let selected = planned_ids(&policy, session.contract.as_ref(), request.checkpoint);
    let mut ran_failure = false;
    let mut saw_unknown = false;
    for check_id in &selected {
        let spec = policy
            .checks
            .iter()
            .find(|check| &check.id == check_id)
            .ok_or_else(|| format!("policy has no check {check_id}"))?;
        let previous = matching_receipt(&session, spec, &facts.candidate_hash, &policy.raw_sha256)
            .map(|receipt| receipt.state.clone());
        if !retry && !needs_run(spec, previous.as_deref()) {
            ran_failure = ran_failure || previous.as_deref() == Some("failed");
            saw_unknown = saw_unknown || previous.as_deref() == Some("unknown");
            if request.checkpoint == Checkpoint::Iteration && (ran_failure || saw_unknown) {
                break;
            }
            continue;
        }
        let receipt = execute_check(
            &facts.canonical,
            spec,
            &facts.candidate_hash,
            &policy.raw_sha256,
            &session.request_sha256,
        )?;
        ran_failure = ran_failure || receipt.state == "failed";
        saw_unknown = saw_unknown || receipt.state == "unknown";
        session.receipts.push(receipt);
        if session.receipts.len() > 500 {
            session.receipts.remove(0);
        }
        if request.checkpoint == Checkpoint::Iteration {
            break;
        }
    }
    let facts = inspect_repo(&request.repo)?;
    session.candidate_hash = facts.candidate_hash.clone();
    session.head = facts.head.clone();
    session.dirty = facts.dirty;
    save_session(&session)?;
    let (outcome, terminal, exit_code, pending, next_actions) = judge(
        &session,
        &policy,
        &facts,
        request.checkpoint,
        ran_failure,
        saw_unknown,
    );
    Ok(finish(
        "verify",
        &session,
        &facts,
        &policy,
        &outcome,
        terminal,
        exit_code,
        pending,
        next_actions,
        None,
        started,
    ))
}

fn report_inner(request: ReportRequest, started: Instant) -> Result<Output, String> {
    validate_session_id(&request.session)?;
    let facts = inspect_repo(&request.repo)?;
    let _lock = lock_session(&facts_key(&facts), &request.session)?;
    let mut session = load_session(&facts, &request.session)?
        .ok_or_else(|| format!("unknown session {}", request.session))?;
    if session.binary_version != VERSION {
        return Err(format!(
            "session is pinned to jacu {}; this binary is {VERSION}",
            session.binary_version
        ));
    }
    if session.repo != facts.canonical.to_string_lossy() {
        return Err("session belongs to a different repository".into());
    }
    let policy = load_policy(&facts.canonical)?;
    session.candidate_hash = facts.candidate_hash.clone();
    session.head = facts.head.clone();
    session.dirty = facts.dirty;
    let (outcome, terminal, exit_code, pending, next_actions) = if session.audit {
        (
            "assessment_complete".to_string(),
            true,
            0,
            contract_gaps(session.contract.as_ref()),
            vec!["Audit records observations. It does not certify the project.".into()],
        )
    } else {
        judge(
            &session,
            &policy,
            &facts,
            Checkpoint::Delivery,
            false,
            false,
        )
    };
    let output = finish(
        "report",
        &session,
        &facts,
        &policy,
        &outcome,
        terminal,
        exit_code,
        pending,
        next_actions,
        None,
        started,
    );
    if request.format == ReportFormat::Markdown && output.exit_code != 4 {
        let value: Value = serde_json::from_str(&output.body).map_err(|error| error.to_string())?;
        return Ok(Output {
            exit_code: output.exit_code,
            body: render_markdown(&value),
        });
    }
    Ok(output)
}

fn judge(
    session: &Session,
    policy: &PolicyState,
    facts: &RepoFacts,
    checkpoint: Checkpoint,
    ran_failure: bool,
    saw_unknown: bool,
) -> (String, bool, i32, Vec<String>, Vec<String>) {
    if session.contract.is_none() {
        return (
            "incomplete".into(),
            false,
            3,
            vec!["outcome contract is missing".into()],
            vec!["Call prepare with --contract-file.".into()],
        );
    }
    let mut gaps = contract_gaps(session.contract.as_ref());
    if !target_matches(session, facts) {
        gaps.push("observed branch/path does not match the recorded delivery target".into());
    }
    let required = obligation_ids(policy, session.contract.as_ref());
    if required.is_empty() {
        gaps.push("no mandatory verification is bound to this task".into());
    }
    if !required.is_empty()
        && required.iter().all(|id| {
            policy
                .checks
                .iter()
                .find(|c| &c.id == id)
                .is_some_and(|c| c.kind == "semantic")
        })
    {
        gaps.push("a semantic assessment alone cannot certify a delivery".into());
    }
    let mut failed = Vec::new();
    let mut unknown = Vec::new();
    let mut missing = Vec::new();
    for id in &required {
        match evidence_state(session, policy, facts, id) {
            "passed" | "reused" => {}
            "failed" => failed.push(id.clone()),
            "unknown" => unknown.push(id.clone()),
            _ => missing.push(id.clone()),
        }
    }
    let weakened = weakened_pending(session, policy);
    if checkpoint == Checkpoint::Delivery && !weakened.is_empty() {
        return (
            "incomplete".into(),
            true,
            3,
            weakened,
            vec!["Restore the original check definition. A changed scope needs a separately authorized task, not a self-approved waiver.".into()],
        );
    }
    if checkpoint == Checkpoint::Iteration {
        let mut pending = gaps;
        pending.extend(missing.iter().map(|id| format!("check {id} has not run")));
        pending.extend(failed.iter().map(|id| format!("check {id} failed")));
        pending.extend(unknown.iter().map(|id| format!("check {id} is unknown")));
        if ran_failure || !failed.is_empty() {
            return (
                "failed".into(),
                false,
                2,
                pending,
                vec!["The failing check was kept. Repair the candidate, or use --retry only after diagnosing a transient external failure.".into()],
            );
        }
        if saw_unknown || !unknown.is_empty() {
            return (
                "incomplete".into(),
                false,
                3,
                pending,
                vec!["Required evidence is unknown. Diagnose it and use --retry for a transient external blocker; otherwise change the candidate.".into()],
            );
        }
        return (
            "ready".into(),
            false,
            0,
            pending,
            vec![
                "Call verify --checkpoint delivery when the slice is in the delivery target."
                    .into(),
            ],
        );
    }
    if !failed.is_empty() || ran_failure {
        let mut pending = gaps;
        pending.extend(failed.iter().map(|id| format!("check {id} failed")));
        return (
            "failed".into(),
            false,
            2,
            pending,
            vec![
                "Repair the failed check. Use --retry only for a diagnosed transient failure."
                    .into(),
            ],
        );
    }
    if !unknown.is_empty() || saw_unknown || !missing.is_empty() || !gaps.is_empty() {
        let no_gaps = gaps.is_empty();
        let mut pending = gaps;
        pending.extend(missing.iter().map(|id| format!("check {id} has not run")));
        pending.extend(unknown.iter().map(|id| format!("check {id} is unknown")));
        let external = unknown
            .iter()
            .any(|id| receipt_is_external(session, policy, facts, id));
        let terminal = external && missing.is_empty() && no_gaps;
        return (
            "incomplete".into(),
            terminal,
            3,
            pending,
            vec![if external {
                "An external blocker stopped the check. The repository was not modified.".into()
            } else {
                "Delivery still has outstanding outcomes or evidence.".into()
            }],
        );
    }
    ("complete".into(), true, 0, Vec::new(), Vec::new())
}

fn obligation_ids(policy: &PolicyState, contract: Option<&Contract>) -> Vec<String> {
    let mut ids = policy.required.clone();
    if let Some(contract) = contract {
        for outcome in &contract.outcomes {
            if outcome.implementation == "not_applicable" && outcome.delivery == "not_applicable" {
                continue;
            }
            for evidence in &outcome.evidence {
                if !ids.contains(evidence) {
                    ids.push(evidence.clone());
                }
            }
        }
    }
    ids
}

fn evidence_state<'a>(
    session: &'a Session,
    policy: &PolicyState,
    facts: &RepoFacts,
    id: &str,
) -> &'a str {
    let Some(spec) = policy.checks.iter().find(|check| check.id == id) else {
        return "unknown";
    };
    matching_receipt(session, spec, &facts.candidate_hash, &policy.raw_sha256)
        .map(|receipt| receipt.state.as_str())
        .unwrap_or("missing")
}

fn matching_receipt<'a>(
    session: &'a Session,
    spec: &CheckSpec,
    candidate_hash: &str,
    policy_sha256: &str,
) -> Option<&'a Receipt> {
    let argv = argv_of(spec);
    let program_hash = runner::program_hash(Path::new(&session.repo), spec)
        .unwrap_or_else(|_| "unavailable".into());
    let environment = env_fingerprint(&spec.environment);
    session.receipts.iter().rev().find(|receipt| {
        receipt.request_sha256 == session.request_sha256
            && receipt.program_sha256 == program_hash
            && receipt.env_sha256 == environment
            && receipt.check_id == spec.id
            && receipt.argv == argv
            && receipt.cwd == spec.cwd
            && receipt.candidate_hash == candidate_hash
            && receipt.policy_sha256 == policy_sha256
            && matches!(
                receipt.state.as_str(),
                "passed" | "reused" | "failed" | "unknown"
            )
    })
}

fn needs_run(spec: &CheckSpec, state: Option<&str>) -> bool {
    match state {
        None => true,
        Some(_) if spec.reuse == "never" || spec.external_state => true,
        Some(_) => false,
    }
}

fn planned_ids(
    policy: &PolicyState,
    contract: Option<&Contract>,
    checkpoint: Checkpoint,
) -> Vec<String> {
    let ids = obligation_ids(policy, contract);
    if ids.is_empty() && checkpoint == Checkpoint::Iteration {
        policy.checks.iter().map(|check| check.id.clone()).collect()
    } else {
        ids
    }
}

fn execute_check(
    repo: &Path,
    spec: &CheckSpec,
    candidate_hash: &str,
    policy_sha256: &str,
    request_sha256: &str,
) -> Result<Receipt, String> {
    runner::execute(
        repo,
        spec,
        candidate_hash,
        policy_sha256,
        request_sha256,
        &snapshot_inputs(repo)?,
    )
}

fn argv_of(spec: &CheckSpec) -> Vec<String> {
    let mut argv = vec![spec.program.clone()];
    argv.extend(spec.args.clone());
    argv
}

fn finish(
    command: &'static str,
    session: &Session,
    facts: &RepoFacts,
    policy: &PolicyState,
    outcome: &str,
    terminal: bool,
    exit_code: i32,
    pending: Vec<String>,
    next_actions: Vec<String>,
    error: Option<String>,
    started: Instant,
) -> Output {
    let mut timings = BTreeMap::new();
    timings.insert("total".to_string(), started.elapsed().as_millis());
    if let Some(receipt) = session
        .receipts
        .iter()
        .rev()
        .find(|receipt| receipt.kind.starts_with("cargo"))
    {
        timings.insert("next_build".to_string(), receipt.duration_ms);
    }
    let checks: Vec<EvidenceView> = policy
        .checks
        .iter()
        .map(|spec| {
            let state = evidence_state(session, policy, facts, &spec.id);
            let matched =
                matching_receipt(session, spec, &facts.candidate_hash, &policy.raw_sha256);
            let detail = matched
                .map(|receipt| receipt.detail.clone())
                .unwrap_or_else(|| "not run for this candidate".into());
            let excerpt = matched
                .map(|receipt| receipt.stdout_excerpt.clone())
                .unwrap_or_default();
            EvidenceView {
                id: spec.id.clone(),
                state: if state == "missing" {
                    "deferred".into()
                } else {
                    state.into()
                },
                detail,
                excerpt,
            }
        })
        .collect();
    let evidence = checks
        .iter()
        .filter(|check| {
            matches!(
                check.state.as_str(),
                "passed" | "reused" | "failed" | "unknown"
            )
        })
        .cloned()
        .collect();
    let envelope = Envelope {
        schema_version: SCHEMA_VERSION,
        binary_version: VERSION,
        command,
        session_id: Some(session.session_id.clone()),
        request_sha256: session.request_sha256.clone(),
        outcome: outcome.to_string(),
        terminal,
        exit_code,
        candidate: CandidateView {
            repo: facts.canonical.to_string_lossy().into_owned(),
            head: facts.head.clone(),
            dirty: facts.dirty,
            hash: facts.candidate_hash.clone(),
            git: facts.git,
        },
        evidence,
        pending,
        next_actions,
        timings_ms: timings,
        error,
        effort: session
            .contract
            .as_ref()
            .map(|contract| effort_view(&contract.effort)),
        inventory: Some(inventory_view(facts)),
        checks,
        outcomes: session
            .contract
            .as_ref()
            .map(|contract| {
                contract
                    .outcomes
                    .iter()
                    .map(|outcome| OutcomeView {
                        id: outcome.id.clone(),
                        implementation: outcome.implementation.clone(),
                        delivery: outcome.delivery.clone(),
                        evidence: outcome.evidence.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        usage: UsageView::unknown(),
        counts_in_denominator: true,
        delivery_target: session.delivery_target.clone(),
        assurance: serde_json::json!({"implementation":"agent_declared","requirements":"agent_mapped_not_independently_certified","checks":"observed_execution","target":"observed_path_and_branch","tamper_proof":false}),
        policy_source: policy.source.clone(),
        discovered_checks: policy.checks.clone(),
    };
    Output {
        exit_code,
        body: serde_json::to_string(&envelope).unwrap_or_else(|_| "{\"exit_code\":4}".into()),
    }
}

fn effort_view(effort: &EffortSpec) -> EffortView {
    let sum =
        u16::from(effort.uncertainty) + u16::from(effort.novelty) + u16::from(effort.coupling);
    let mut recommended = match sum {
        0 | 1 => "focused",
        2..=4 => "standard",
        _ => "deep",
    };
    if effort.consequence == "critical" {
        recommended = "deep";
    } else if matches!(effort.consequence.as_str(), "sensitive" | "unknown")
        && recommended == "focused"
    {
        recommended = "standard";
    }
    EffortView {
        uncertainty: effort.uncertainty,
        novelty: effort.novelty,
        coupling: effort.coupling,
        consequence: effort.consequence.clone(),
        recommended_effort: recommended.to_string(),
        applied_effort: None,
        application_status: "advisory_only".into(),
    }
}

fn inventory_view(facts: &RepoFacts) -> InventoryView {
    InventoryView {
        git: facts.git,
        toplevel: facts.canonical.to_string_lossy().into_owned(),
        head: facts.head.clone(),
        branch: facts.branch.clone(),
        dirty: facts.dirty,
        worktrees: facts.worktrees.clone(),
        notes: facts.notes.clone(),
        disk_logical_bytes: facts.disk_logical_bytes,
        disk_allocated_bytes: facts.disk_allocated_bytes,
        disk_scope: "snapshot inputs only; not a repository-wide disk scan".into(),
    }
}

fn render_markdown(value: &Value) -> String {
    let outcome = value
        .get("outcome")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let session = value
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let mut lines = vec![
        "# Jacu report".to_string(),
        format!("Session: {session}"),
        format!("Outcome: {outcome}"),
    ];
    if let Some(candidate) = value.get("candidate") {
        lines.push(format!(
            "Candidate: {} dirty={}",
            candidate.get("hash").and_then(Value::as_str).unwrap_or(""),
            candidate
                .get("dirty")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        ));
    }
    lines.push("".into());
    lines.push("## Pending".into());
    push_string_list(&mut lines, value.get("pending"));
    lines.push("".into());
    lines.push("## Next".into());
    push_string_list(&mut lines, value.get("next_actions"));
    lines.push("".into());
    lines.push("## Evidence".into());
    if let Some(items) = value.get("evidence").and_then(Value::as_array) {
        if items.is_empty() {
            lines.push("- none".into());
        }
        for item in items {
            lines.push(format!(
                "- {}: {}",
                item.get("id").and_then(Value::as_str).unwrap_or(""),
                item.get("state").and_then(Value::as_str).unwrap_or("")
            ));
        }
    }
    lines.push("".into());
    lines.join("\n")
}

fn push_string_list(lines: &mut Vec<String>, value: Option<&Value>) {
    let Some(items) = value.and_then(Value::as_array) else {
        lines.push("- none".into());
        return;
    };
    if items.is_empty() {
        lines.push("- none".into());
        return;
    }
    for item in items {
        lines.push(format!("- {}", item.as_str().unwrap_or("")));
    }
}

fn contract_gaps(contract: Option<&Contract>) -> Vec<String> {
    let Some(contract) = contract else {
        return Vec::new();
    };
    let mut gaps = Vec::new();
    for outcome in &contract.outcomes {
        if outcome.implementation == "pending" || outcome.delivery == "pending" {
            gaps.push(format!("outcome {} is still pending", outcome.id));
        }
        let active =
            !(outcome.implementation == "not_applicable" && outcome.delivery == "not_applicable");
        if active && outcome.evidence.is_empty() {
            gaps.push(format!("outcome {} has no evidence", outcome.id));
        }
    }
    gaps
}

fn dropped_outcomes(previous: &Contract, next: &Contract) -> Vec<String> {
    let kept: BTreeSet<&str> = next
        .outcomes
        .iter()
        .map(|outcome| outcome.id.as_str())
        .collect();
    let allowed: BTreeSet<&str> = next
        .amendments
        .iter()
        .flat_map(|amendment| amendment.removes.iter().map(String::as_str))
        .collect();
    previous
        .outcomes
        .iter()
        .filter(|outcome| {
            !kept.contains(outcome.id.as_str()) && !allowed.contains(outcome.id.as_str())
        })
        .map(|outcome| outcome.id.clone())
        .collect()
}

fn read_contract(path: &Path) -> Result<Contract, String> {
    let bytes = read_limited(path, 262144)?;
    let contract: Contract =
        serde_json::from_slice(&bytes).map_err(|error| format!("invalid contract: {error}"))?;
    if contract.schema_version != SCHEMA_VERSION {
        return Err("contract schema_version must be 1".into());
    }
    if contract.outcomes.is_empty() {
        return Err("contract needs at least one outcome".into());
    }
    let mut seen = BTreeSet::new();
    for outcome in &contract.outcomes {
        validate_id(&outcome.id, "outcome")?;
        if !seen.insert(outcome.id.clone()) {
            return Err(format!("duplicate outcome {}", outcome.id));
        }
        if outcome.statement.trim().is_empty() || outcome.statement.chars().count() > 2000 {
            return Err(format!(
                "outcome {} statement must be 1 to 2000 characters",
                outcome.id
            ));
        }
        for field in [&outcome.implementation, &outcome.delivery] {
            if !matches!(
                field.as_str(),
                "pending" | "present" | "not_applicable" | "in_target"
            ) {
                return Err(format!("outcome {} has an invalid state", outcome.id));
            }
        }
        if outcome.implementation == "in_target" || outcome.delivery == "present" {
            return Err(format!(
                "outcome {} swaps implementation and delivery states",
                outcome.id
            ));
        }
        if (outcome.implementation == "not_applicable" || outcome.delivery == "not_applicable")
            && outcome
                .inapplicable_reason
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
        {
            return Err(format!(
                "outcome {} needs an inapplicable_reason",
                outcome.id
            ));
        }
    }
    validate_effort(&contract.effort)?;
    for amendment in &contract.amendments {
        if !amendment.removes.is_empty() || !amendment.weakens.is_empty() {
            return Err("self-declared amendments cannot remove or weaken bound obligations; retain the original task and record any authorized scope change separately".into());
        }
        validate_id(&amendment.id, "amendment")?;
        if amendment.reason.trim().is_empty() || amendment.source.trim().is_empty() {
            return Err(format!(
                "amendment {} needs a reason and a source",
                amendment.id
            ));
        }
    }
    Ok(contract)
}

fn validate_contract(contract: &Contract, policy: &PolicyState, repo: &Path) -> Result<(), String> {
    validate_context(contract, repo)?;
    for outcome in &contract.outcomes {
        for evidence in &outcome.evidence {
            if !policy.checks.iter().any(|check| &check.id == evidence) {
                return Err(format!(
                    "outcome {} cites evidence {evidence} that is not in .jacu-fast.json",
                    outcome.id
                ));
            }
        }
    }
    Ok(())
}

fn validate_effort(effort: &EffortSpec) -> Result<(), String> {
    for (name, score) in [
        ("uncertainty", effort.uncertainty),
        ("novelty", effort.novelty),
        ("coupling", effort.coupling),
    ] {
        if score > 2 {
            return Err(format!("{name} must be 0, 1, or 2"));
        }
    }
    if !matches!(
        effort.consequence.as_str(),
        "ordinary" | "sensitive" | "critical" | "unknown"
    ) {
        return Err("consequence must be ordinary, sensitive, critical, or unknown".into());
    }
    Ok(())
}

fn load_policy(repo: &Path) -> Result<PolicyState, String> {
    let path = repo.join(".jacu-fast.json");
    let configured = path.is_file();
    let policy = if configured {
        serde_json::from_slice::<PolicyFile>(&read_limited(&path, 262144)?)
            .map_err(|error| format!("invalid .jacu-fast.json: {error}"))?
    } else {
        discover_policy(repo)?
    };
    let bytes = serde_json::to_vec(&policy).map_err(|e| e.to_string())?;
    if policy.schema_version != SCHEMA_VERSION {
        return Err(".jacu-fast.json schema_version must be 1".into());
    }
    let mut seen = BTreeSet::new();
    for check in &policy.checks {
        validate_id(&check.id, "check")?;
        if !seen.insert(check.id.clone()) {
            return Err(format!("duplicate check {}", check.id));
        }
        if check.program.trim().is_empty()
            || check.program.starts_with('-')
            || check.program.contains('\0')
        {
            return Err(format!("check {} has an invalid program", check.id));
        }
        if check.timeout_seconds == 0 || check.timeout_seconds > 7200 {
            return Err(format!(
                "check {} timeout must be 1 to 7200 seconds",
                check.id
            ));
        }
        if !matches!(
            check.reuse.as_str(),
            "same-session-declared-inputs" | "never"
        ) {
            return Err(format!("check {} has an unsupported reuse value", check.id));
        }
        for arg in &check.args {
            if arg.contains('\0') {
                return Err(format!("check {} has an invalid argument", check.id));
            }
        }
        if !check.minimum_score.is_finite() || !(0.0..=1.0).contains(&check.minimum_score) {
            return Err("invalid semantic threshold".into());
        }
        for input in &check.inputs {
            let _ = within(repo, input)?;
        }
        for key in &check.environment {
            if key.is_empty() || !key.chars().all(|x| x.is_ascii_alphanumeric() || x == '_') {
                return Err("invalid environment key".into());
            }
        }
        resolve_cwd(repo, &check.cwd)?;
        let cwd = resolve_cwd(repo, &check.cwd)?;
        resolve_program(&cwd, &check.program)?;
    }
    for required in &policy.delivery.required_check_ids {
        if !policy.checks.iter().any(|check| &check.id == required) {
            return Err(format!("required check {required} is not defined"));
        }
    }
    Ok(PolicyState {
        raw_sha256: sha256_hex(&bytes),
        required: policy.delivery.required_check_ids,
        checks: policy.checks,
        source: if configured {
            ".jacu-fast.json"
        } else {
            "discovered from existing project manifests"
        }
        .into(),
    })
}

fn resolve_cwd(repo: &Path, cwd: &str) -> Result<PathBuf, String> {
    if cwd.contains('\0') {
        return Err("check cwd is invalid".into());
    }
    let candidate = if Path::new(cwd).is_absolute() {
        PathBuf::from(cwd)
    } else {
        repo.join(cwd)
    };
    let canonical = fs::canonicalize(&candidate)
        .map_err(|error| format!("check cwd is not available: {error}"))?;
    if !canonical.starts_with(repo) {
        return Err("check cwd escapes the repository".into());
    }
    Ok(canonical)
}

fn resolve_program(cwd: &Path, program: &str) -> Result<PathBuf, String> {
    let path = Path::new(program);
    if path.components().count() > 1 || program.contains('/') || program.contains('\\') {
        let candidate = if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        };
        let canonical = fs::canonicalize(&candidate)
            .map_err(|error| format!("check program is not available: {error}"))?;
        if !path.is_absolute() && !canonical.starts_with(cwd) {
            return Err("relative check program escapes the repository".into());
        }
        return Ok(canonical);
    }
    Ok(PathBuf::from(program))
}

fn inspect_repo(requested: &Path) -> Result<RepoFacts, String> {
    let root = inventory::root(requested)?;
    let facts = inventory::inspect(&root, &snapshot_inputs(&root)?)?;
    Ok(RepoFacts {
        canonical: facts.root,
        git: facts.git,
        head: facts.head,
        branch: facts.branch,
        dirty: facts.dirty,
        worktrees: facts.worktrees,
        notes: facts.notes,
        candidate_hash: facts.hash,
        disk_logical_bytes: facts.logical_bytes,
        disk_allocated_bytes: facts.allocated_bytes,
    })
}

fn data_root() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("JACU_HOME") {
        return Ok(PathBuf::from(path));
    }
    if let Some(path) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(path).join(".jacu-fast"));
    }
    if let Some(path) = std::env::var_os("USERPROFILE") {
        return Ok(PathBuf::from(path).join(".jacu-fast"));
    }
    Err("home directory is not available; set JACU_HOME".into())
}

fn facts_key(facts: &RepoFacts) -> String {
    let full = sha256_hex(facts.canonical.to_string_lossy().as_bytes());
    full[..32].to_string()
}

fn session_path(repo_key: &str, session_id: &str) -> Result<PathBuf, String> {
    Ok(data_root()?
        .join("repos")
        .join(repo_key)
        .join("sessions")
        .join(format!("{session_id}.json")))
}

fn load_session(facts: &RepoFacts, session_id: &str) -> Result<Option<Session>, String> {
    let path = session_path(&facts_key(facts), session_id)?;
    if !path.exists() {
        return Ok(None);
    }
    let bytes = read_limited(&path, 4194304)?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| "session record is corrupt".to_string())
}

fn save_session(session: &Session) -> Result<(), String> {
    let path = session_path(&session.repo_key, &session.session_id)?;
    let parent = path.parent().ok_or("session path has no parent")?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create session directory: {error}"))?;
    let bytes = serde_json::to_vec_pretty(session).map_err(|error| error.to_string())?;
    let tmp = parent.join(format!(
        ".{}.tmp-{}-{}",
        session.session_id,
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&tmp)
            .map_err(|error| format!("cannot write session: {error}"))?;
        file.write_all(&bytes)
            .map_err(|error| format!("cannot write session: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("cannot persist session: {error}"))?;
    }
    fs::rename(&tmp, &path).map_err(|error| format!("cannot commit session: {error}"))?;
    Ok(())
}

fn lock_session(repo_key: &str, session_id: &str) -> Result<Lock, String> {
    let dir = data_root()?.join("locks");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join(format!("{repo_key}-{session_id}.lock")))
        .map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        let start = Instant::now();
        loop {
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
                break;
            }
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::EWOULDBLOCK) {
                return Err(error.to_string());
            }
            if start.elapsed() > Duration::from_secs(5) {
                return Err("session lock is held by another jacu process".into());
            }
            thread::sleep(Duration::from_millis(25));
        }
    }
    #[cfg(not(unix))]
    {
        return Err(
            "runtime locking requires macOS or Linux; use skill-only mode on this platform".into(),
        );
    }
    #[allow(unreachable_code)]
    Ok(Lock { file })
}

fn validate_session_id(id: &str) -> Result<(), String> {
    let ok = !id.is_empty()
        && id.len() <= 80
        && id
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphanumeric())
        && id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'));
    if ok {
        Ok(())
    } else {
        Err("session id must be 1 to 80 letters, numbers, dots, underscores, or hyphens".into())
    }
}

fn validate_id(id: &str, label: &str) -> Result<(), String> {
    let ok = !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'));
    if ok {
        Ok(())
    } else {
        Err(format!("{label} id is invalid"))
    }
}

fn new_session_id(request_hash: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut hasher = Sha256::new();
    hasher.update(request_hash.as_bytes());
    hasher.update(nanos.to_le_bytes());
    hasher.update(std::process::id().to_le_bytes());
    let encoded = sha256_hex(hasher.finalize().as_slice());
    format!("s{}", &encoded[..16])
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0xf) as usize] as char);
    }
    encoded
}

impl UsageView {
    fn unknown() -> Self {
        Self {
            input_tokens: None,
            output_tokens: None,
            cost: None,
        }
    }
}

fn env_fingerprint(keys: &[String]) -> String {
    runner::environment_hash(keys)
}

fn minimum_map(policy: &PolicyState, contract: &Contract) -> BTreeMap<String, u32> {
    obligation_ids(policy, Some(contract))
        .into_iter()
        .map(|id| {
            let minimum = policy
                .checks
                .iter()
                .find(|check| check.id == id)
                .map(|check| check.minimum_tests)
                .unwrap_or(0);
            (id, minimum)
        })
        .collect()
}

fn weaken_conflict(
    bound: &BTreeMap<String, u32>,
    next: &BTreeMap<String, u32>,
    contract: &Contract,
) -> Option<String> {
    let allowed: BTreeSet<&str> = contract
        .amendments
        .iter()
        .flat_map(|amendment| amendment.weakens.iter().map(String::as_str))
        .collect();
    for (id, previous) in bound {
        match next.get(id) {
            None if !allowed.contains(id.as_str()) => {
                return Some(format!("check {id} was removed without an amendment"));
            }
            Some(current) if current < previous && !allowed.contains(id.as_str()) => {
                return Some(format!(
                    "check {id} assertion was weakened without an amendment"
                ));
            }
            _ => {}
        }
    }
    None
}

fn weakened_pending(session: &Session, policy: &PolicyState) -> Vec<String> {
    let Some(contract) = session.contract.as_ref() else {
        return Vec::new();
    };
    let allowed: BTreeSet<&str> = contract
        .amendments
        .iter()
        .flat_map(|amendment| amendment.weakens.iter().map(String::as_str))
        .collect();
    let mut pending = Vec::new();
    if session
        .bound_required
        .iter()
        .any(|id| !policy.required.contains(id))
    {
        pending.push("mandatory delivery check was removed".into());
    }
    for (id, bound) in &session.bound_checks {
        if policy
            .checks
            .iter()
            .find(|c| &c.id == id)
            .is_some_and(|c| check_definition(c) != *bound)
        {
            pending.push(format!("bound check {id} definition was changed"));
        }
    }
    for (id, bound) in &session.bound_minimums {
        match policy.checks.iter().find(|check| &check.id == id) {
            None if !allowed.contains(id.as_str()) => {
                pending.push(format!("check {id} was removed without an amendment"));
            }
            Some(check) if check.minimum_tests < *bound && !allowed.contains(id.as_str()) => {
                pending.push(format!(
                    "check {id} assertion was weakened without an amendment"
                ));
            }
            _ => {}
        }
    }
    pending
}

fn validate_context(contract: &Contract, repo: &Path) -> Result<(), String> {
    for item in &contract.context {
        let opaque = item.id.starts_with('f')
            && item.id.len() > 1
            && item.id[1..].chars().all(|ch| ch.is_ascii_digit());
        let shaped = item
            .id
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphabetic())
            && item
                .id
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':' | '-'));
        if opaque || !shaped {
            return Err(format!("context id {} is stale or ambiguous", item.id));
        }
        if item.path.starts_with('/') || item.path.split(['/', '\\']).any(|part| part == "..") {
            return Err(format!("context path {} escapes the repository", item.path));
        }
        if !within(repo, &item.path)?.is_file() {
            return Err(format!(
                "context path {} is not in the current tree",
                item.path
            ));
        }
    }
    Ok(())
}

fn receipt_is_external(
    session: &Session,
    policy: &PolicyState,
    facts: &RepoFacts,
    id: &str,
) -> bool {
    let Some(spec) = policy.checks.iter().find(|check| &check.id == id) else {
        return false;
    };
    matching_receipt(session, spec, &facts.candidate_hash, &policy.raw_sha256)
        .map(|receipt| receipt.external)
        .unwrap_or(false)
}

fn write_diagnostic(session: &Session) -> Result<(), String> {
    let dir = data_root()?.join("diagnostics").join(&session.repo_key);
    fs::create_dir_all(&dir).map_err(|error| format!("cannot create diagnostics: {error}"))?;
    let path = dir.join(format!("{}.json", session.session_id));
    let bytes = serde_json::to_vec(&serde_json::json!({
        "session_id": session.session_id,
        "candidate_hash": session.candidate_hash,
    }))
    .map_err(|error| error.to_string())?;
    fs::write(&path, &bytes).map_err(|error| format!("cannot write diagnostic: {error}"))?;
    fs::write(
        dir.join(format!("{}.owned", session.session_id)),
        sha256_hex(&bytes),
    )
    .map_err(|error| format!("cannot write diagnostic owner record: {error}"))?;
    Ok(())
}

pub fn capabilities() -> Output {
    let body = serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "binary_version": VERSION,
        "command": "capabilities",
        "outcome": "ready",
        "exit_code": 0,
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "effort_application": "advisory_only",
        "snapshot": "tracked_and_untracked_content_v2",
        "delivery_target": "pinned_path_and_branch",
        "compiles_on_missing_binary": false,
        "commands": ["prepare", "verify", "report", "clean", "capabilities"],
        "usage": {"input_tokens": null, "output_tokens": null, "cost": null}
    });
    Output {
        exit_code: 0,
        body: body.to_string(),
    }
}

pub struct HookRequest {
    pub repo: PathBuf,
    pub session: String,
}

pub fn verify_hook(request: HookRequest) -> Output {
    if std::env::var("JACU_HOOK").ok().as_deref() == Some("1") {
        let body = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "binary_version": VERSION,
            "command": "verify",
            "session_id": request.session,
            "outcome": "incomplete",
            "terminal": true,
            "exit_code": 0,
            "pending": ["hook recursion stopped"],
            "next_actions": [],
            "usage": {"input_tokens": null, "output_tokens": null, "cost": null},
            "counts_in_denominator": true
        });
        return Output {
            exit_code: 0,
            body: body.to_string(),
        };
    }
    let started = Instant::now();
    match verify_hook_inner(request, started) {
        Ok(output) => output,
        Err(message) => invalid_output("verify", &message),
    }
}

fn verify_hook_inner(request: HookRequest, started: Instant) -> Result<Output, String> {
    validate_session_id(&request.session)?;
    let facts = inspect_repo(&request.repo)?;
    let _lock = lock_session(&facts_key(&facts), &request.session)?;
    let session = load_session(&facts, &request.session)?
        .ok_or_else(|| format!("unknown session {}", request.session))?;
    if session.binary_version != VERSION {
        return Err(format!(
            "session is pinned to jacu {}; this binary is {VERSION}",
            session.binary_version
        ));
    }
    let policy = load_policy(&facts.canonical)?;
    let (outcome, terminal, exit_code, pending, next_actions) = if session.audit {
        (
            "assessment_complete".to_string(),
            true,
            0,
            Vec::new(),
            vec!["Hook evaluation did not run project commands.".into()],
        )
    } else {
        judge(
            &session,
            &policy,
            &facts,
            Checkpoint::Delivery,
            false,
            false,
        )
    };
    Ok(finish(
        "verify",
        &session,
        &facts,
        &policy,
        &outcome,
        terminal,
        exit_code,
        pending,
        next_actions,
        None,
        started,
    ))
}

pub fn clean(repo: PathBuf) -> Output {
    match clean_inner(&repo) {
        Ok(output) => output,
        Err(message) => invalid_output("clean", &message),
    }
}

fn clean_inner(repo: &Path) -> Result<Output, String> {
    let start = Instant::now();
    let root = inventory::root(repo)?;
    let key = sha256_hex(root.to_string_lossy().as_bytes())[..32].to_string();
    let dir = data_root()?.join("diagnostics").join(&key);
    let ttl = std::env::var("JACU_DIAGNOSTIC_TTL_SECONDS")
        .ok()
        .and_then(|x| x.parse::<u64>().ok())
        .unwrap_or(604800);
    let mut removed = Vec::new();
    let mut reclaimed = 0u64;
    if dir.exists() {
        if fs::symlink_metadata(&dir)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("diagnostics directory must not be a symlink".into());
        }
        for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("active-")
                || !name.ends_with(".json")
                || !entry.file_type().map_err(|e| e.to_string())?.is_file()
            {
                continue;
            }
            let id = name.trim_end_matches(".json");
            if validate_id(id, "session").is_err() {
                continue;
            }
            let owned = dir.join(format!("{id}.owned"));
            if fs::symlink_metadata(&owned)
                .map(|m| !m.is_file() || m.file_type().is_symlink())
                .unwrap_or(true)
            {
                continue;
            }
            let Ok(body) = read_limited(&path, 8192) else {
                continue;
            };
            let Ok(hash) = fs::read_to_string(&owned) else {
                continue;
            };
            if sha256_hex(&body) != hash.trim() {
                continue;
            }
            let old = entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|d| d.as_secs() >= ttl);
            if !old {
                continue;
            }
            // Diagnostics contain only regenerable identifiers, never work or receipts.
            if fs::remove_file(&path).is_ok() {
                let _ = fs::remove_file(owned);
                reclaimed += body.len() as u64;
                removed.push(name);
            }
        }
    }
    Ok(Output { exit_code: 0, body: serde_json::json!({"schema_version":SCHEMA_VERSION,"binary_version":VERSION,"command":"clean","outcome":"complete","terminal":true,
        "removed":removed,"reclaimed_bytes":reclaimed,"pending":[],"next_actions":[],"timings_ms":{"total":start.elapsed().as_millis()},"usage":serde_json::json!({"input_tokens":null,"output_tokens":null,"cost":null}),"counts_in_denominator":true}).to_string() })
}

fn empty_candidate() -> CandidateView {
    CandidateView {
        repo: String::new(),
        head: None,
        dirty: false,
        hash: String::new(),
        git: false,
    }
}

fn discover_policy(root: &Path) -> Result<PolicyFile, String> {
    let mut checks = Vec::new();
    let make = |id: &str, kind: &str, program: &str, args: Vec<&str>| -> CheckSpec {
        CheckSpec {
            id: id.into(),
            kind: kind.into(),
            program: program.into(),
            args: args.into_iter().map(String::from).collect(),
            cwd: ".".into(),
            timeout_seconds: 600,
            minimum_tests: if kind.contains("test") { 1 } else { 0 },
            reuse: "never".into(),
            external_state: false,
            environment: Vec::new(),
            inputs: Vec::new(),
            minimum_score: score_default(),
        }
    };
    if root.join("Cargo.toml").is_file() {
        checks.push(make(
            "cargo-tests",
            "cargo-test",
            "cargo",
            vec!["test", "--locked"],
        ));
    }
    if root.join("go.mod").is_file() {
        checks.push(make(
            "go-tests",
            "go-test",
            "go",
            vec!["test", "-json", "./..."],
        ));
    }
    if root.join("Package.swift").is_file() {
        checks.push(make("swift-tests", "swift-test", "swift", vec!["test"]));
    }
    if root.join("package.json").is_file() {
        let manifest: Value =
            serde_json::from_slice(&read_limited(&root.join("package.json"), 256 * 1024)?)
                .map_err(|e| format!("invalid package.json: {e}"))?;
        let manager = if root.join("pnpm-lock.yaml").is_file() {
            "pnpm"
        } else if root.join("yarn.lock").is_file() {
            "yarn"
        } else if root.join("bun.lock").is_file() || root.join("bun.lockb").is_file() {
            "bun"
        } else {
            "npm"
        };
        for key in ["typecheck", "lint", "test"] {
            if manifest["scripts"][key]
                .as_str()
                .is_some_and(|v| !v.trim().is_empty())
            {
                checks.push(make(
                    &format!("node-{key}"),
                    if key == "test" {
                        "node-test"
                    } else {
                        "external"
                    },
                    manager,
                    vec!["run", key],
                ));
            }
        }
    }
    let ids = checks.iter().map(|c| c.id.clone()).collect();
    Ok(PolicyFile {
        schema_version: SCHEMA_VERSION,
        checks,
        delivery: DeliverySpec {
            required_check_ids: ids,
        },
    })
}

fn check_definition(check: &CheckSpec) -> String {
    sha256_hex(&serde_json::to_vec(check).unwrap_or_default())
}
fn target_matches(session: &Session, facts: &RepoFacts) -> bool {
    session
        .delivery_target
        .as_ref()
        .is_some_and(|t| Path::new(&t.path) == facts.canonical && t.branch == facts.branch)
}
fn validate_revision(previous: &Contract, next: &Contract) -> Result<(), String> {
    for old in &previous.outcomes {
        let new = next
            .outcomes
            .iter()
            .find(|o| o.id == old.id)
            .ok_or_else(|| format!("contract drops outcome {}", old.id))?;
        if old.statement != new.statement
            || old.source_quote != new.source_quote
            || old.inapplicable_reason != new.inapplicable_reason
        {
            return Err(format!("bound outcome {} was rewritten", old.id));
        }
        if old.evidence.iter().any(|id| !new.evidence.contains(id))
            || (old.implementation != "not_applicable" && new.implementation == "not_applicable")
        {
            return Err(format!("bound outcome {} was weakened", old.id));
        }
    }
    Ok(())
}
