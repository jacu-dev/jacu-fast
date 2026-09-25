use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const SCHEMA_VERSION: u32 = 1;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const OUTPUT_CAP: usize = 262_144;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Contract {
    schema_version: u32,
    outcomes: Vec<OutcomeSpec>,
    effort: EffortSpec,
    #[serde(default)]
    amendments: Vec<Amendment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OutcomeSpec {
    id: String,
    statement: String,
    implementation: String,
    delivery: String,
    #[serde(default)]
    evidence: Vec<String>,
    #[serde(default)]
    inapplicable_reason: Option<String>,
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
    reason: String,
    source: String,
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
}

#[derive(Debug, Clone, Deserialize)]
struct PolicyFile {
    #[serde(default = "schema_one")]
    schema_version: u32,
    #[serde(default)]
    checks: Vec<CheckSpec>,
    #[serde(default)]
    delivery: DeliverySpec,
}

#[derive(Debug, Clone, Deserialize)]
struct CheckSpec {
    id: String,
    #[serde(default = "external_kind")]
    #[allow(dead_code)]
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
}

#[derive(Debug, Clone, Deserialize, Default)]
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
}

#[derive(Debug, Clone, Serialize)]
struct EvidenceView {
    id: String,
    state: String,
    detail: String,
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
}

struct PolicyState {
    raw_sha256: String,
    checks: Vec<CheckSpec>,
    required: Vec<String>,
}

struct Lock {
    path: PathBuf,
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
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
    let started = Instant::now();
    match verify_inner(request, started) {
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
    let request_bytes = fs::read(&request.request_file)
        .map_err(|error| format!("cannot read request file: {error}"))?;
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
    let mut session = load_session(&facts, &session_id)?.unwrap_or(Session {
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
    });
    if session.schema_version != SCHEMA_VERSION {
        return Err("session schema is not compatible with this binary".into());
    }
    if session.repo != facts.canonical.to_string_lossy() {
        return Err("session belongs to a different repository".into());
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
        if let Some(previous) = &session.contract {
            let dropped = dropped_outcomes(previous, &next);
            if !dropped.is_empty() {
                return Err(format!(
                    "contract drops outcomes without an amendment: {}",
                    dropped.join(", ")
                ));
            }
        }
        validate_contract(&next, &policy)?;
        session.contract = Some(next);
    }
    save_session(&session)?;
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
            vec!["Write a contract file and call prepare again with --contract-file and --session.".into()],
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

fn verify_inner(request: VerifyRequest, started: Instant) -> Result<Output, String> {
    validate_session_id(&request.session)?;
    let facts = inspect_repo(&request.repo)?;
    let _lock = lock_session(&facts_key(&facts), &request.session)?;
    let mut session = load_session(&facts, &request.session)?
        .ok_or_else(|| format!("unknown session {}", request.session))?;
    if session.schema_version != SCHEMA_VERSION {
        return Err("session schema is not compatible with this binary".into());
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
        if !needs_run(spec, previous.as_deref()) {
            ran_failure = ran_failure || previous.as_deref() == Some("failed");
            saw_unknown = saw_unknown || previous.as_deref() == Some("unknown");
            if request.checkpoint == Checkpoint::Iteration && (ran_failure || saw_unknown) {
                break;
            }
            continue;
        }
        let receipt = execute_check(&facts.canonical, spec, &facts.candidate_hash, &policy.raw_sha256)?;
        ran_failure = ran_failure || receipt.state == "failed";
        saw_unknown = saw_unknown || receipt.state == "unknown";
        session.receipts.push(receipt);
        if request.checkpoint == Checkpoint::Iteration {
            break;
        }
    }
    save_session(&session)?;
    let (outcome, terminal, exit_code, pending, next_actions) =
        judge(&session, &policy, &facts, request.checkpoint, ran_failure, saw_unknown);
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
    let gaps = contract_gaps(session.contract.as_ref());
    let required = obligation_ids(policy, session.contract.as_ref());
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
                vec!["The failing check was kept. Change the candidate before it can run again.".into()],
            );
        }
        if saw_unknown || !unknown.is_empty() {
            return (
                "incomplete".into(),
                false,
                3,
                pending,
                vec!["Required evidence is unknown. An unchanged candidate will not rerun it.".into()],
            );
        }
        return (
            "ready".into(),
            false,
            0,
            pending,
            vec!["Call verify --checkpoint delivery when the slice is in the delivery target.".into()],
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
            vec!["Repair the failed check. An unchanged candidate will not rerun it.".into()],
        );
    }
    if !unknown.is_empty() || saw_unknown || !missing.is_empty() || !gaps.is_empty() {
        let mut pending = gaps;
        pending.extend(missing.iter().map(|id| format!("check {id} has not run")));
        pending.extend(unknown.iter().map(|id| format!("check {id} is unknown")));
        return (
            "incomplete".into(),
            false,
            3,
            pending,
            vec!["Delivery still has outstanding outcomes or evidence.".into()],
        );
    }
    (
        "complete".into(),
        true,
        0,
        Vec::new(),
        Vec::new(),
    )
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
    session.receipts.iter().rev().find(|receipt| {
        receipt.check_id == spec.id
            && receipt.argv == argv
            && receipt.cwd == spec.cwd
            && receipt.candidate_hash == candidate_hash
            && receipt.policy_sha256 == policy_sha256
            && matches!(receipt.state.as_str(), "passed" | "reused" | "failed" | "unknown")
    })
}

fn needs_run(spec: &CheckSpec, state: Option<&str>) -> bool {
    match state {
        None => true,
        Some("passed") if spec.reuse == "never" => true,
        Some(_) => false,
    }
}

fn planned_ids(policy: &PolicyState, contract: Option<&Contract>, checkpoint: Checkpoint) -> Vec<String> {
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
) -> Result<Receipt, String> {
    let cwd = resolve_cwd(repo, &spec.cwd)?;
    let program = resolve_program(&cwd, &spec.program)?;
    let started = Instant::now();
    let mut command = Command::new(&program);
    command
        .args(&spec.args)
        .current_dir(&cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return Ok(finished_receipt(
                spec,
                "unknown",
                None,
                candidate_hash,
                policy_sha256,
                0,
                Vec::new(),
                format!("cannot start check: {error}").into_bytes(),
                false,
                format!("cannot start check: {error}"),
            ));
        }
    };
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_handle = thread::spawn(move || read_capped(stdout));
    let stderr_handle = thread::spawn(move || read_capped(stderr));
    let limit = Duration::from_secs(spec.timeout_seconds.max(1));
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if started.elapsed() > limit => {
                force_kill(&mut child);
                let _ = child.wait();
                break None;
            }
            Ok(None) => thread::sleep(Duration::from_millis(30)),
            Err(error) => {
                force_kill(&mut child);
                let _ = child.wait();
                let stdout = stdout_handle.join().unwrap_or((Vec::new(), true));
                let stderr = stderr_handle.join().unwrap_or((Vec::new(), true));
                return Ok(finished_receipt(
                    spec,
                    "unknown",
                    None,
                    candidate_hash,
                    policy_sha256,
                    started.elapsed().as_millis(),
                    stdout.0,
                    stderr.0,
                    true,
                    format!("check wait failed: {error}"),
                ));
            }
        }
    };
    let (stdout, stdout_truncated) = stdout_handle.join().unwrap_or((Vec::new(), true));
    let (stderr, stderr_truncated) = stderr_handle.join().unwrap_or((Vec::new(), true));
    let truncated = stdout_truncated || stderr_truncated;
    let duration = started.elapsed().as_millis();
    if status.is_none() {
        return Ok(finished_receipt(
            spec,
            "unknown",
            None,
            candidate_hash,
            policy_sha256,
            duration,
            stdout,
            stderr,
            true,
            "check timed out".to_string(),
        ));
    }
    let code = status.and_then(|status| status.code());
    let combined = {
        let mut all = stdout.clone();
        all.extend_from_slice(&stderr);
        String::from_utf8_lossy(&all).into_owned()
    };
    let (state, detail) = classify_check(spec, code, &combined, truncated);
    Ok(finished_receipt(
        spec,
        state,
        code,
        candidate_hash,
        policy_sha256,
        duration,
        stdout,
        stderr,
        truncated,
        detail,
    ))
}

fn classify_check(spec: &CheckSpec, code: Option<i32>, output: &str, truncated: bool) -> (&'static str, String) {
    if truncated {
        return ("unknown", "check output was truncated".into());
    }
    if code != Some(0) {
        return (
            "failed",
            format!("check exited with {}", code.map(|code| code.to_string()).unwrap_or_else(|| "signal".into())),
        );
    }
    if spec.minimum_tests > 0 {
        if output.contains("running 0 tests") {
            return ("failed", "filter selected zero tests".into());
        }
        match passed_count(output) {
            Some(count) if count >= spec.minimum_tests => {}
            Some(count) => {
                return (
                    "failed",
                    format!("passed {count} tests; minimum is {}", spec.minimum_tests),
                );
            }
            None => return ("failed", "minimum test count was not present in the output".into()),
        }
    }
    ("passed", "check passed".into())
}

fn passed_count(output: &str) -> Option<u32> {
    for line in output.lines() {
        let Some(index) = line.find(" passed") else {
            continue;
        };
        let prefix = line[..index].trim_end();
        let digits: String = prefix
            .chars()
            .rev()
            .take_while(|ch| ch.is_ascii_digit())
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        if !digits.is_empty() {
            return digits.parse().ok();
        }
    }
    None
}

fn finished_receipt(
    spec: &CheckSpec,
    state: &str,
    exit_code: Option<i32>,
    candidate_hash: &str,
    policy_sha256: &str,
    duration_ms: u128,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    truncated: bool,
    detail: String,
) -> Receipt {
    Receipt {
        check_id: spec.id.clone(),
        state: state.to_string(),
        exit_code,
        candidate_hash: candidate_hash.to_string(),
        argv: argv_of(spec),
        cwd: spec.cwd.clone(),
        policy_sha256: policy_sha256.to_string(),
        duration_ms,
        stdout_sha256: sha256_hex(&stdout),
        stderr_sha256: sha256_hex(&stderr),
        stdout_excerpt: excerpt(&stdout),
        stderr_excerpt: excerpt(&stderr),
        truncated,
        detail,
    }
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
    let checks: Vec<EvidenceView> = policy
        .checks
        .iter()
        .map(|spec| {
            let state = evidence_state(session, policy, facts, &spec.id);
            let detail = matching_receipt(session, spec, &facts.candidate_hash, &policy.raw_sha256)
                .map(|receipt| receipt.detail.clone())
                .unwrap_or_else(|| "not run for this candidate".into());
            EvidenceView {
                id: spec.id.clone(),
                state: if state == "missing" { "deferred".into() } else { state.into() },
                detail,
            }
        })
        .collect();
    let evidence = checks
        .iter()
        .filter(|check| matches!(check.state.as_str(), "passed" | "reused" | "failed" | "unknown"))
        .cloned()
        .collect();
    let envelope = Envelope {
        schema_version: SCHEMA_VERSION,
        binary_version: VERSION,
        command,
        session_id: Some(session.session_id.clone()),
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
        effort: session.contract.as_ref().map(|contract| effort_view(&contract.effort)),
        inventory: Some(inventory_view(facts)),
        checks,
    };
    Output {
        exit_code,
        body: serde_json::to_string(&envelope).unwrap_or_else(|_| "{\"exit_code\":4}".into()),
    }
}

fn effort_view(effort: &EffortSpec) -> EffortView {
    let sum = u16::from(effort.uncertainty) + u16::from(effort.novelty) + u16::from(effort.coupling);
    let mut recommended = match sum {
        0 | 1 => "focused",
        2..=4 => "standard",
        _ => "deep",
    };
    if effort.consequence == "critical" {
        recommended = "deep";
    } else if matches!(effort.consequence.as_str(), "sensitive" | "unknown") && recommended == "focused"
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
    }
}

fn render_markdown(value: &Value) -> String {
    let outcome = value.get("outcome").and_then(Value::as_str).unwrap_or("unknown");
    let session = value.get("session_id").and_then(Value::as_str).unwrap_or("");
    let mut lines = vec![
        "# Jacu report".to_string(),
        format!("Session: {session}"),
        format!("Outcome: {outcome}"),
    ];
    if let Some(candidate) = value.get("candidate") {
        lines.push(format!(
            "Candidate: {} dirty={}",
            candidate.get("hash").and_then(Value::as_str).unwrap_or(""),
            candidate.get("dirty").and_then(Value::as_bool).unwrap_or(false)
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
        let active = !(outcome.implementation == "not_applicable" && outcome.delivery == "not_applicable");
        if active && outcome.evidence.is_empty() {
            gaps.push(format!("outcome {} has no evidence", outcome.id));
        }
    }
    gaps
}

fn dropped_outcomes(previous: &Contract, next: &Contract) -> Vec<String> {
    let kept: BTreeSet<&str> = next.outcomes.iter().map(|outcome| outcome.id.as_str()).collect();
    let allowed: BTreeSet<&str> = next
        .amendments
        .iter()
        .flat_map(|amendment| amendment.removes.iter().map(String::as_str))
        .collect();
    previous
        .outcomes
        .iter()
        .filter(|outcome| !kept.contains(outcome.id.as_str()) && !allowed.contains(outcome.id.as_str()))
        .map(|outcome| outcome.id.clone())
        .collect()
}

fn read_contract(path: &Path) -> Result<Contract, String> {
    let bytes = fs::read(path).map_err(|error| format!("cannot read contract file: {error}"))?;
    let contract: Contract = serde_json::from_slice(&bytes).map_err(|error| format!("invalid contract: {error}"))?;
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
            return Err(format!("outcome {} statement must be 1 to 2000 characters", outcome.id));
        }
        for field in [&outcome.implementation, &outcome.delivery] {
            if !matches!(field.as_str(), "pending" | "present" | "not_applicable" | "in_target") {
                return Err(format!("outcome {} has an invalid state", outcome.id));
            }
        }
        if outcome.implementation == "in_target" || outcome.delivery == "present" {
            return Err(format!("outcome {} swaps implementation and delivery states", outcome.id));
        }
        if (outcome.implementation == "not_applicable" || outcome.delivery == "not_applicable")
            && outcome.inapplicable_reason.as_deref().unwrap_or("").trim().is_empty()
        {
            return Err(format!("outcome {} needs an inapplicable_reason", outcome.id));
        }
    }
    validate_effort(&contract.effort)?;
    for amendment in &contract.amendments {
        validate_id(&amendment.id, "amendment")?;
        if amendment.reason.trim().is_empty() || amendment.source.trim().is_empty() {
            return Err(format!("amendment {} needs a reason and a source", amendment.id));
        }
    }
    Ok(contract)
}

fn validate_contract(contract: &Contract, policy: &PolicyState) -> Result<(), String> {
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
    if !matches!(effort.consequence.as_str(), "ordinary" | "sensitive" | "critical" | "unknown") {
        return Err("consequence must be ordinary, sensitive, critical, or unknown".into());
    }
    Ok(())
}

fn load_policy(repo: &Path) -> Result<PolicyState, String> {
    let path = repo.join(".jacu-fast.json");
    if !path.is_file() {
        return Ok(PolicyState {
            raw_sha256: "none".into(),
            checks: Vec::new(),
            required: Vec::new(),
        });
    }
    let bytes = fs::read(&path).map_err(|error| format!("cannot read .jacu-fast.json: {error}"))?;
    let policy: PolicyFile = serde_json::from_slice(&bytes).map_err(|error| format!("invalid .jacu-fast.json: {error}"))?;
    if policy.schema_version != SCHEMA_VERSION {
        return Err(".jacu-fast.json schema_version must be 1".into());
    }
    let mut seen = BTreeSet::new();
    for check in &policy.checks {
        validate_id(&check.id, "check")?;
        if !seen.insert(check.id.clone()) {
            return Err(format!("duplicate check {}", check.id));
        }
        if check.program.trim().is_empty() || check.program.starts_with('-') || check.program.contains('\0') {
            return Err(format!("check {} has an invalid program", check.id));
        }
        if check.timeout_seconds == 0 || check.timeout_seconds > 7200 {
            return Err(format!("check {} timeout must be 1 to 7200 seconds", check.id));
        }
        if !matches!(check.reuse.as_str(), "same-session-declared-inputs" | "never") {
            return Err(format!("check {} has an unsupported reuse value", check.id));
        }
        for arg in &check.args {
            if arg.contains('\0') {
                return Err(format!("check {} has an invalid argument", check.id));
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
    let canonical = fs::canonicalize(&candidate).map_err(|error| format!("check cwd is not available: {error}"))?;
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
        let canonical = fs::canonicalize(&candidate).map_err(|error| format!("check program is not available: {error}"))?;
        if !path.is_absolute() && !canonical.starts_with(cwd) {
            return Err("relative check program escapes the repository".into());
        }
        return Ok(canonical);
    }
    Ok(PathBuf::from(program))
}

fn inspect_repo(requested: &Path) -> Result<RepoFacts, String> {
    if !requested.exists() {
        return Err("repository path does not exist".into());
    }
    if !requested.is_dir() {
        return Err("repository path is not a directory".into());
    }
    let canonical = fs::canonicalize(requested).map_err(|error| format!("cannot read repository path: {error}"))?;
    let mut notes = Vec::new();
    let toplevel = git_bytes(&canonical, &["rev-parse", "--show-toplevel"]).ok();
    let root = if let Some(bytes) = toplevel.as_ref() {
        let text = String::from_utf8_lossy(bytes).trim().to_string();
        let path = fs::canonicalize(&text).map_err(|error| format!("git toplevel is not readable: {error}"))?;
        if path != canonical {
            notes.push(format!(
                "requested path is inside {path}",
                path = path.display()
            ));
        }
        path
    } else {
        notes.push("git is not available for this path".into());
        canonical.clone()
    };
    let git = toplevel.is_some();
    let head = git_bytes(&root, &["rev-parse", "HEAD"])
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).trim().to_string())
        .filter(|value| !value.is_empty());
    let branch = git_bytes(&root, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).trim().to_string())
        .filter(|value| !value.is_empty() && value != "HEAD");
    let status_bytes = git_bytes(&root, &["status", "--porcelain=v1", "-z"]).unwrap_or_default();
    let dirty = status_bytes.iter().any(|byte| !byte.is_ascii_whitespace());
    let worktrees = git_bytes(&root, &["worktree", "list", "--porcelain", "-z"])
        .map(|bytes| parse_worktrees(&bytes))
        .unwrap_or_default();
    if git && head.is_none() {
        notes.push("git HEAD is not available".into());
    }
    let mut identity = Vec::new();
    identity.extend_from_slice(head.as_deref().unwrap_or("").as_bytes());
    identity.push(0);
    identity.extend_from_slice(&status_bytes);
    Ok(RepoFacts {
        candidate_hash: sha256_hex(&identity),
        canonical: root,
        git,
        head,
        branch,
        dirty,
        worktrees,
        notes,
    })
}

fn parse_worktrees(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .split('\0')
        .filter_map(|field| field.strip_prefix("worktree ").map(str::to_string))
        .collect()
}

fn git_bytes(repo: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .env("GIT_PAGER", "cat");
    let child = command.spawn().map_err(|error| format!("cannot run git: {error}"))?;
    let output = child.wait_with_output().map_err(|error| format!("git failed: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
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
    let bytes = fs::read(&path).map_err(|error| format!("cannot read session: {error}"))?;
    serde_json::from_slice(&bytes).map(Some).map_err(|_| "session record is corrupt".to_string())
}

fn save_session(session: &Session) -> Result<(), String> {
    let path = session_path(&session.repo_key, &session.session_id)?;
    let parent = path.parent().ok_or("session path has no parent")?;
    fs::create_dir_all(parent).map_err(|error| format!("cannot create session directory: {error}"))?;
    let bytes = serde_json::to_vec_pretty(session).map_err(|error| error.to_string())?;
    let tmp = parent.join(format!(
        ".{}.tmp-{}",
        session.session_id,
        std::process::id()
    ));
    {
        let mut file = fs::File::create(&tmp).map_err(|error| format!("cannot write session: {error}"))?;
        file.write_all(&bytes).map_err(|error| format!("cannot write session: {error}"))?;
        file.sync_all().ok();
    }
    fs::rename(&tmp, &path).map_err(|error| format!("cannot commit session: {error}"))?;
    Ok(())
}

fn lock_session(repo_key: &str, session_id: &str) -> Result<Lock, String> {
    let dir = data_root()?.join("locks");
    fs::create_dir_all(&dir).map_err(|error| format!("cannot create lock directory: {error}"))?;
    let path = dir.join(format!("{repo_key}-{session_id}.lock"));
    for _ in 0..40 {
        match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                let _ = writeln!(file, "{}", std::process::id());
                return Ok(Lock { path });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if let Ok(text) = fs::read_to_string(&path) {
                    if let Ok(pid) = text.trim().parse::<i32>() {
                        if !pid_alive(pid) {
                            let _ = fs::remove_file(&path);
                            continue;
                        }
                    }
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(error) => return Err(format!("cannot lock session: {error}")),
        }
    }
    Err("session lock is held by another jacu process".into())
}

fn pid_alive(pid: i32) -> bool {
    if pid <= 0 {
        return false;
    }
    #[cfg(unix)]
    {
        let rc = unsafe { libc::kill(pid, 0) };
        if rc == 0 {
            return true;
        }
        return std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM);
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn validate_session_id(id: &str) -> Result<(), String> {
    let ok = !id.is_empty()
        && id.len() <= 80
        && id.chars().next().is_some_and(|ch| ch.is_ascii_alphanumeric())
        && id.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'));
    if ok {
        Ok(())
    } else {
        Err("session id must be 1 to 80 letters, numbers, dots, underscores, or hyphens".into())
    }
}

fn validate_id(id: &str, label: &str) -> Result<(), String> {
    let ok = !id.is_empty()
        && id.len() <= 64
        && id.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'));
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

fn excerpt(bytes: &[u8]) -> String {
    let take = bytes.len().min(1200);
    String::from_utf8_lossy(&bytes[..take]).into_owned()
}

fn read_capped(pipe: Option<impl Read>) -> (Vec<u8>, bool) {
    let mut reader = match pipe {
        Some(reader) => reader,
        None => return (Vec::new(), false),
    };
    let mut kept = Vec::new();
    let mut buffer = [0_u8; 8192];
    let mut truncated = false;
    loop {
        let read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => count,
            Err(_) => {
                truncated = true;
                break;
            }
        };
        if kept.len() < OUTPUT_CAP {
            let room = OUTPUT_CAP - kept.len();
            let take = read.min(room);
            kept.extend_from_slice(&buffer[..take]);
            if take < read {
                truncated = true;
            }
        } else {
            truncated = true;
        }
    }
    (kept, truncated)
}

fn force_kill(child: &mut Child) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.kill();
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
