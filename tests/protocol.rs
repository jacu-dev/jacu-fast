use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_jacu")
}

fn run(home: &Path, args: &[&str]) -> (i32, Value, String) {
    let output = Command::new(bin())
        .args(args)
        .env("JACU_HOME", home)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("run jacu");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let code = output.status.code().unwrap_or(1);
    let value = serde_json::from_str(&stdout).unwrap_or_else(|_| {
        panic!(
            "stdout was not json (exit {code}): {stdout}\nstderr {}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (code, value, stdout)
}

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(repo)
        .env("GIT_AUTHOR_NAME", "Jacu")
        .env("GIT_AUTHOR_EMAIL", "jacu@example.com")
        .env("GIT_COMMITTER_NAME", "Jacu")
        .env("GIT_COMMITTER_EMAIL", "jacu@example.com")
        .status()
        .expect("git");
    assert!(status.success(), "git {args:?}");
}

fn repo(path: &Path) {
    fs::create_dir_all(path).unwrap();
    git(path, &["init", "-b", "main"]);
    fs::write(path.join("README"), "synthetic\n").unwrap();
    git(path, &["add", "README"]);
    git(path, &["commit", "-m", "init"]);
}

fn write_exe(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }
}

fn temp(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "jacu-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn contract(evidence: &str, implementation: &str) -> String {
    format!(
        r#"{{
  "schema_version": 1,
  "outcomes": [{{
    "id": "O1",
    "statement": "The synthetic check ran.",
    "implementation": "{implementation}",
    "delivery": "in_target",
    "evidence": ["{evidence}"]
  }}],
  "effort": {{"uncertainty": 0, "novelty": 0, "coupling": 1, "consequence": "ordinary"}}
}}"#
    )
}

#[test]
fn missing_arguments_are_invalid_input() {
    let home = temp("home-args");
    let (code, value, _) = run(&home, &["prepare"]);
    assert_eq!(code, 4);
    assert_eq!(value["exit_code"], 4);
    assert_eq!(value["outcome"], "unavailable");
}

#[test]
fn prepare_is_ready_and_audit_runs_no_command() {
    let root = temp("audit");
    let home = root.join("home");
    let repo_path = root.join("repo");
    repo(&repo_path);
    let counter = root.join("count");
    let script = root.join("tick");
    write_exe(
        &script,
        "#!/bin/sh\nprintf x >> \"$1\"\nexit 0\n",
    );
    let policy = format!(
        r#"{{
  "schema_version": 1,
  "checks": [{{
    "id": "tick",
    "kind": "external",
    "program": "{}",
    "args": ["{}"],
    "cwd": ".",
    "timeout_seconds": 5
  }}],
  "delivery": {{"required_check_ids": ["tick"]}}
}}"#,
        script.display(),
        counter.display()
    );
    fs::write(repo_path.join(".jacu-fast.json"), policy).unwrap();
    let request = root.join("request.txt");
    fs::write(&request, "Audit the synthetic repository.\n").unwrap();
    let (code, value, _) = run(
        &home,
        &[
            "prepare",
            "--repo",
            repo_path.to_str().unwrap(),
            "--request-file",
            request.to_str().unwrap(),
            "--audit",
        ],
    );
    assert_eq!(code, 0, "{value}");
    assert_eq!(value["outcome"], "assessment_complete");
    assert!(value["terminal"].as_bool().unwrap());
    assert!(!counter.exists());
    assert!(value["session_id"].as_str().unwrap().starts_with('s'));
}

#[test]
fn delivery_completes_once_and_reuses_the_receipt() {
    let root = temp("delivery");
    let home = root.join("home");
    let repo_path = root.join("repo");
    repo(&repo_path);
    let counter = root.join("count");
    let script = root.join("tick");
    write_exe(&script, "#!/bin/sh\nprintf x >> \"$1\"\nexit 0\n");
    fs::write(
        repo_path.join(".jacu-fast.json"),
        format!(
            r#"{{
  "schema_version": 1,
  "checks": [{{
    "id": "tick",
    "kind": "external",
    "program": "{}",
    "args": ["{}"],
    "cwd": ".",
    "timeout_seconds": 5,
    "reuse": "same-session-declared-inputs"
  }}],
  "delivery": {{"required_check_ids": ["tick"]}}
}}"#,
            script.display(),
            counter.display()
        ),
    )
    .unwrap();
    let request = root.join("request.txt");
    let contract_path = root.join("contract.json");
    fs::write(&request, "Run the synthetic check.\n").unwrap();
    fs::write(&contract_path, contract("tick", "present")).unwrap();
    let (code, prepared, _) = run(
        &home,
        &[
            "prepare",
            "--repo",
            repo_path.to_str().unwrap(),
            "--request-file",
            request.to_str().unwrap(),
            "--contract-file",
            contract_path.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{prepared}");
    assert_eq!(prepared["outcome"], "ready");
    let session = prepared["session_id"].as_str().unwrap();
    let verify_args = [
        "verify",
        "--repo",
        repo_path.to_str().unwrap(),
        "--session",
        session,
        "--checkpoint",
        "delivery",
    ];
    let (code, first, _) = run(&home, &verify_args);
    assert_eq!(code, 0, "{first}");
    assert_eq!(first["outcome"], "complete");
    assert_eq!(fs::read(&counter).unwrap(), b"x");
    let (code, second, _) = run(&home, &verify_args);
    assert_eq!(code, 0, "{second}");
    assert_eq!(second["outcome"], "complete");
    assert_eq!(fs::read(&counter).unwrap(), b"x");
}

#[test]
fn unchanged_failure_is_not_rerun() {
    let root = temp("fail");
    let home = root.join("home");
    let repo_path = root.join("repo");
    repo(&repo_path);
    let counter = root.join("count");
    let script = root.join("tick");
    write_exe(&script, "#!/bin/sh\nprintf x >> \"$1\"\nexit 1\n");
    fs::write(
        repo_path.join(".jacu-fast.json"),
        format!(
            r#"{{
  "schema_version": 1,
  "checks": [{{
    "id": "tick",
    "kind": "external",
    "program": "{}",
    "args": ["{}"],
    "cwd": ".",
    "timeout_seconds": 5
  }}],
  "delivery": {{"required_check_ids": ["tick"]}}
}}"#,
            script.display(),
            counter.display()
        ),
    )
    .unwrap();
    let request = root.join("request.txt");
    let contract_path = root.join("contract.json");
    fs::write(&request, "Expect a failure.\n").unwrap();
    fs::write(&contract_path, contract("tick", "present")).unwrap();
    let (_, prepared, _) = run(
        &home,
        &[
            "prepare",
            "--repo",
            repo_path.to_str().unwrap(),
            "--request-file",
            request.to_str().unwrap(),
            "--contract-file",
            contract_path.to_str().unwrap(),
        ],
    );
    let session = prepared["session_id"].as_str().unwrap().to_string();
    let args = [
        "verify",
        "--repo",
        repo_path.to_str().unwrap(),
        "--session",
        session.as_str(),
        "--checkpoint",
        "delivery",
    ];
    let (code, _, _) = run(&home, &args);
    assert_eq!(code, 2);
    let (code, second, _) = run(&home, &args);
    assert_eq!(code, 2, "{second}");
    assert_eq!(second["outcome"], "failed");
    assert_eq!(fs::read(&counter).unwrap(), b"x");
}

#[test]
fn zero_tests_do_not_pass() {
    let root = temp("zero");
    let home = root.join("home");
    let repo_path = root.join("repo");
    repo(&repo_path);
    let script = root.join("zero");
    write_exe(&script, "#!/bin/sh\nprintf 'running 0 tests\\n'\nexit 0\n");
    fs::write(
        repo_path.join(".jacu-fast.json"),
        format!(
            r#"{{
  "schema_version": 1,
  "checks": [{{
    "id": "unit",
    "kind": "cargo-test",
    "program": "{}",
    "args": [],
    "cwd": ".",
    "timeout_seconds": 5,
    "minimum_tests": 1
  }}],
  "delivery": {{"required_check_ids": ["unit"]}}
}}"#,
            script.display()
        ),
    )
    .unwrap();
    let request = root.join("request.txt");
    let contract_path = root.join("contract.json");
    fs::write(&request, "Need one test.\n").unwrap();
    fs::write(&contract_path, contract("unit", "present")).unwrap();
    let (_, prepared, _) = run(
        &home,
        &[
            "prepare",
            "--repo",
            repo_path.to_str().unwrap(),
            "--request-file",
            request.to_str().unwrap(),
            "--contract-file",
            contract_path.to_str().unwrap(),
        ],
    );
    let (code, value, _) = run(
        &home,
        &[
            "verify",
            "--repo",
            repo_path.to_str().unwrap(),
            "--session",
            prepared["session_id"].as_str().unwrap(),
            "--checkpoint",
            "delivery",
        ],
    );
    assert_eq!(code, 2, "{value}");
    assert_eq!(value["checks"][0]["state"], "failed");
}

#[test]
fn pending_outcome_stays_incomplete() {
    let root = temp("pending");
    let home = root.join("home");
    let repo_path = root.join("repo");
    repo(&repo_path);
    let script = root.join("ok");
    write_exe(&script, "#!/bin/sh\nexit 0\n");
    fs::write(
        repo_path.join(".jacu-fast.json"),
        format!(
            r#"{{
  "schema_version": 1,
  "checks": [{{
    "id": "ok",
    "kind": "external",
    "program": "{}",
    "args": [],
    "cwd": ".",
    "timeout_seconds": 5
  }}],
  "delivery": {{"required_check_ids": ["ok"]}}
}}"#,
            script.display()
        ),
    )
    .unwrap();
    let request = root.join("request.txt");
    let contract_path = root.join("contract.json");
    fs::write(&request, "Not done yet.\n").unwrap();
    fs::write(&contract_path, contract("ok", "pending")).unwrap();
    let (_, prepared, _) = run(
        &home,
        &[
            "prepare",
            "--repo",
            repo_path.to_str().unwrap(),
            "--request-file",
            request.to_str().unwrap(),
            "--contract-file",
            contract_path.to_str().unwrap(),
        ],
    );
    let (code, value, _) = run(
        &home,
        &[
            "verify",
            "--repo",
            repo_path.to_str().unwrap(),
            "--session",
            prepared["session_id"].as_str().unwrap(),
            "--checkpoint",
            "delivery",
        ],
    );
    assert_eq!(code, 3, "{value}");
    assert_eq!(value["outcome"], "incomplete");
}

#[test]
fn dropped_outcome_without_amendment_is_rejected() {
    let root = temp("drop");
    let home = root.join("home");
    let repo_path = root.join("repo");
    repo(&repo_path);
    let request = root.join("request.txt");
    fs::write(&request, "Keep both outcomes.\n").unwrap();
    let first = root.join("first.json");
    fs::write(
        &first,
        r#"{"schema_version":1,"outcomes":[
          {"id":"O1","statement":"One.","implementation":"pending","delivery":"pending","evidence":[]},
          {"id":"O2","statement":"Two.","implementation":"pending","delivery":"pending","evidence":[]}
        ],"effort":{"uncertainty":1,"novelty":1,"coupling":1,"consequence":"ordinary"}}"#,
    )
    .unwrap();
    let (_, prepared, _) = run(
        &home,
        &[
            "prepare",
            "--repo",
            repo_path.to_str().unwrap(),
            "--request-file",
            request.to_str().unwrap(),
            "--contract-file",
            first.to_str().unwrap(),
        ],
    );
    let session = prepared["session_id"].as_str().unwrap();
    let second = root.join("second.json");
    fs::write(
        &second,
        r#"{"schema_version":1,"outcomes":[
          {"id":"O1","statement":"One.","implementation":"pending","delivery":"pending","evidence":[]}
        ],"effort":{"uncertainty":1,"novelty":1,"coupling":1,"consequence":"ordinary"}}"#,
    )
    .unwrap();
    let (code, value, _) = run(
        &home,
        &[
            "prepare",
            "--repo",
            repo_path.to_str().unwrap(),
            "--request-file",
            request.to_str().unwrap(),
            "--session",
            session,
            "--contract-file",
            second.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 4, "{value}");
    assert!(value["error"].as_str().unwrap().contains("O2"));
}

#[test]
fn timeout_is_not_a_pass() {
    let root = temp("timeout");
    let home = root.join("home");
    let repo_path = root.join("slow repo");
    repo(&repo_path);
    fs::write(
        repo_path.join(".jacu-fast.json"),
        r#"{"schema_version":1,"checks":[{"id":"slow","kind":"external","program":"/bin/sleep","args":["5"],"cwd":".","timeout_seconds":1}],"delivery":{"required_check_ids":["slow"]}}"#,
    )
    .unwrap();
    let request = root.join("request.txt");
    let contract_path = root.join("contract.json");
    fs::write(&request, "Time box the sleep.\n").unwrap();
    fs::write(&contract_path, contract("slow", "present")).unwrap();
    let (_, prepared, _) = run(
        &home,
        &[
            "prepare",
            "--repo",
            repo_path.to_str().unwrap(),
            "--request-file",
            request.to_str().unwrap(),
            "--contract-file",
            contract_path.to_str().unwrap(),
        ],
    );
    let started = Instant::now();
    let (code, value, _) = run(
        &home,
        &[
            "verify",
            "--repo",
            repo_path.to_str().unwrap(),
            "--session",
            prepared["session_id"].as_str().unwrap(),
            "--checkpoint",
            "delivery",
        ],
    );
    assert!(started.elapsed().as_secs() < 4, "timeout did not stop the check");
    assert_eq!(code, 3, "{value}");
    assert_ne!(value["checks"][0]["state"], "passed");
}

#[test]
fn report_markdown_names_the_outcome() {
    let root = temp("markdown");
    let home = root.join("home");
    let repo_path = root.join("repo");
    repo(&repo_path);
    let request = root.join("request.txt");
    fs::write(&request, "Read only.\n").unwrap();
    let (_, prepared, _) = run(
        &home,
        &[
            "prepare",
            "--repo",
            repo_path.to_str().unwrap(),
            "--request-file",
            request.to_str().unwrap(),
            "--audit",
        ],
    );
    let output = Command::new(bin())
        .args([
            "report",
            "--repo",
            repo_path.to_str().unwrap(),
            "--session",
            prepared["session_id"].as_str().unwrap(),
            "--format",
            "markdown",
        ])
        .env("JACU_HOME", &home)
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("# Jacu report"));
    assert!(text.contains("assessment_complete"));
}
