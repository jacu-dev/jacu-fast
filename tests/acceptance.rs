use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_jacu")
}

fn run(home: &Path, args: &[&str]) -> (i32, Value) {
    let output = Command::new(bin())
        .args(args)
        .env("JACU_HOME", home)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .output()
        .expect("jacu");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let code = output.status.code().unwrap_or(1);
    let value = serde_json::from_str(&stdout).unwrap_or_else(|_| {
        panic!(
            "not json ({code}): {stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (code, value)
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
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

fn repo(path: &Path) {
    fs::create_dir_all(path).unwrap();
    git(path, &["init", "-b", "main"]);
    fs::write(path.join("README"), "synthetic\n").unwrap();
    git(path, &["add", "README"]);
    git(path, &["commit", "-m", "init"]);
}

fn exe(path: &Path, body: &str) {
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
        "jacu-acc-{name}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn policy(repo: &Path, body: &str) {
    fs::write(repo.join(".jacu-fast.json"), body).unwrap();
}

fn contract(evidence: &str, implementation: &str, delivery: &str) -> String {
    format!(
        r#"{{"schema_version":1,"outcomes":[{{"id":"O1","statement":"Observed behavior.","implementation":"{implementation}","delivery":"{delivery}","evidence":["{evidence}"]}}],"effort":{{"uncertainty":0,"novelty":0,"coupling":0,"consequence":"ordinary"}}}}"#
    )
}

struct Task {
    root: PathBuf,
    home: PathBuf,
    repo: PathBuf,
    request: PathBuf,
}

fn task(name: &str) -> Task {
    let root = temp(name);
    let repo_path = root.join("repo");
    repo(&repo_path);
    Task {
        home: root.join("home"),
        request: root.join("request.txt"),
        repo: repo_path,
        root,
    }
}

impl Task {
    fn prepare(&self, extra: &[&str]) -> (i32, Value) {
        fs::write(&self.request, "Synthetic request.\n").unwrap();
        let mut args = vec![
            "prepare",
            "--repo",
            self.repo.to_str().unwrap(),
            "--request-file",
            self.request.to_str().unwrap(),
        ];
        args.extend_from_slice(extra);
        run(&self.home, &args)
    }

    fn verify(&self, session: &str, checkpoint: &str) -> (i32, Value) {
        run(
            &self.home,
            &[
                "verify",
                "--repo",
                self.repo.to_str().unwrap(),
                "--session",
                session,
                "--checkpoint",
                checkpoint,
            ],
        )
    }
}

#[test]
fn ni01_open_stdin_does_not_wait() {
    let task = task("ni01");
    fs::write(&task.request, "Synthetic request.\n").unwrap();
    let mut child = Command::new(bin())
        .args([
            "prepare",
            "--repo",
            task.repo.to_str().unwrap(),
            "--request-file",
            task.request.to_str().unwrap(),
        ])
        .env("JACU_HOME", &task.home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let started = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "prepare waited on stdin"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn ni02_closed_stdin_without_policy_is_structured() {
    let task = task("ni02");
    let (code, value) = task.prepare(&[]);
    assert_eq!(code, 0, "{value}");
    assert_eq!(value["outcome"], "ready");
    assert!(value.get("error").is_none() || value["error"].is_null());
}

#[test]
fn ni03_child_question_gets_no_answer() {
    let task = task("ni03");
    let script = task.root.join("ask");
    exe(
        &script,
        "#!/bin/sh\nif read -r line; then echo answered; exit 0; else echo eof; exit 2; fi\n",
    );
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"ask","kind":"external","program":"{}","args":[],"cwd":".","timeout_seconds":5}}],"delivery":{{"required_check_ids":["ask"]}}}}"#,
            script.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("ask", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let (code, value) = task.verify(prepared["session_id"].as_str().unwrap(), "delivery");
    assert_ne!(code, 0);
    assert_ne!(value["checks"][0]["state"], "passed");
    assert!(!value["checks"][0]["detail"]
        .as_str()
        .unwrap_or("")
        .contains("answered"));
}

#[test]
fn ni04_large_streams_are_bounded() {
    let task = task("ni04");
    let script = task.root.join("flood");
    exe(
        &script,
        "#!/bin/sh\nread -r _ || true\npython3 - <<'PY'\nimport sys\nchunk = b'x'*100000\nfor _ in range(5):\n    sys.stdout.buffer.write(chunk)\n    sys.stderr.buffer.write(chunk)\nPY\nexit 0\n",
    );
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"flood","kind":"external","program":"{}","args":[],"cwd":".","timeout_seconds":8}}],"delivery":{{"required_check_ids":["flood"]}}}}"#,
            script.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("flood", "present", "in_target")).unwrap();
    let started = Instant::now();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let (code, value) = task.verify(prepared["session_id"].as_str().unwrap(), "delivery");
    assert!(started.elapsed() < Duration::from_secs(8));
    assert_ne!(value["checks"][0]["state"], "passed", "{value}");
    assert_ne!(code, 0);
}

#[test]
fn ni05_no_editor_or_login_ui() {
    let task = task("ni05");
    let script = task.root.join("env");
    exe(&script, "#!/bin/sh\nprintf '%s' \"$GIT_EDITOR\"\nexit 0\n");
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"env","kind":"external","program":"{}","args":[],"cwd":".","timeout_seconds":5}}],"delivery":{{"required_check_ids":["env"]}}}}"#,
            script.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("env", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let (code, value) = task.verify(prepared["session_id"].as_str().unwrap(), "delivery");
    assert_eq!(code, 0, "{value}");
    let text = value.to_string();
    assert!(!text.to_lowercase().contains("password"));
    assert!(!text.contains("login required"));
}

#[test]
fn ni06_unchanged_failure_is_terminal_to_report() {
    let task = task("ni06");
    let counter = task.root.join("count");
    let script = task.root.join("fail");
    exe(&script, "#!/bin/sh\nprintf x >> \"$1\"\nexit 1\n");
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"fail","kind":"external","program":"{}","args":["{}"],"cwd":".","timeout_seconds":5}}],"delivery":{{"required_check_ids":["fail"]}}}}"#,
            script.display(),
            counter.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("fail", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let session = prepared["session_id"].as_str().unwrap().to_string();
    assert_eq!(task.verify(&session, "delivery").0, 2);
    assert_eq!(task.verify(&session, "delivery").0, 2);
    assert_eq!(fs::read(&counter).unwrap(), b"x");
    let (code, report) = run(
        &task.home,
        &[
            "report",
            "--repo",
            task.repo.to_str().unwrap(),
            "--session",
            &session,
        ],
    );
    assert_eq!(code, 2, "{report}");
    assert_eq!(report["outcome"], "failed");
    assert_eq!(report["counts_in_denominator"], true);
}

#[test]
fn ni07_external_denial_is_not_a_permission_rewrite() {
    let task = task("ni07");
    let mode = fs::metadata(&task.repo).unwrap().permissions();
    policy(
        &task.repo,
        r#"{"schema_version":1,"checks":[{"id":"missing","kind":"external","program":"jacu-no-such-tool","args":[],"cwd":".","timeout_seconds":5}],"delivery":{"required_check_ids":["missing"]}}"#,
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("missing", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let (_, value) = task.verify(prepared["session_id"].as_str().unwrap(), "delivery");
    assert_eq!(value["outcome"], "incomplete");
    assert_eq!(value["terminal"], true);
    assert!(!value["next_actions"].to_string().contains('?'));
    assert_eq!(fs::metadata(&task.repo).unwrap().permissions(), mode);
}

#[test]
fn v01_iteration_runs_one_check() {
    let task = task("v01");
    let first = task.root.join("first");
    let second = task.root.join("second");
    let script = task.root.join("tick");
    exe(&script, "#!/bin/sh\nprintf x >> \"$1\"\nexit 0\n");
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"one","kind":"external","program":"{}","args":["{}"],"cwd":".","timeout_seconds":5}},{{"id":"two","kind":"external","program":"{}","args":["{}"],"cwd":".","timeout_seconds":5}}],"delivery":{{"required_check_ids":["one","two"]}}}}"#,
            script.display(),
            first.display(),
            script.display(),
            second.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(
        &contract_path,
        r#"{"schema_version":1,"outcomes":[{"id":"O1","statement":"Both checks.","implementation":"present","delivery":"in_target","evidence":["one","two"]}],"effort":{"uncertainty":0,"novelty":0,"coupling":0,"consequence":"ordinary"}}"#,
    )
    .unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let (_, value) = task.verify(prepared["session_id"].as_str().unwrap(), "iteration");
    assert_eq!(value["outcome"], "ready", "{value}");
    assert_eq!(fs::read(&first).unwrap(), b"x");
    assert!(!second.exists());
}

#[test]
fn v02_zero_tests_do_not_pass() {
    let task = task("v02");
    let script = task.root.join("zero");
    exe(&script, "#!/bin/sh\nprintf 'running 0 tests\\n'\nexit 0\n");
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"unit","kind":"cargo-test","program":"{}","args":[],"cwd":".","timeout_seconds":5,"minimum_tests":1}}],"delivery":{{"required_check_ids":["unit"]}}}}"#,
            script.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("unit", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let (code, value) = task.verify(prepared["session_id"].as_str().unwrap(), "delivery");
    assert_eq!(code, 2, "{value}");
    assert_eq!(value["checks"][0]["state"], "failed");
}

#[test]
fn v03_timeout_and_truncation_do_not_pass() {
    let task = task("v03");
    policy(
        &task.repo,
        r#"{"schema_version":1,"checks":[{"id":"slow","kind":"external","program":"/bin/sleep","args":["5"],"cwd":".","timeout_seconds":1}],"delivery":{"required_check_ids":["slow"]}}"#,
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("slow", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let (code, value) = task.verify(prepared["session_id"].as_str().unwrap(), "delivery");
    assert_eq!(code, 3, "{value}");
    assert_ne!(value["checks"][0]["state"], "passed");
}

#[test]
fn v04_success_is_reused_once() {
    let task = task("v04");
    let counter = task.root.join("count");
    let script = task.root.join("ok");
    exe(&script, "#!/bin/sh\nprintf x >> \"$1\"\nexit 0\n");
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"ok","kind":"external","program":"{}","args":["{}"],"cwd":".","timeout_seconds":5}}],"delivery":{{"required_check_ids":["ok"]}}}}"#,
            script.display(),
            counter.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("ok", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let session = prepared["session_id"].as_str().unwrap();
    assert_eq!(task.verify(session, "delivery").0, 0);
    assert_eq!(task.verify(session, "delivery").0, 0);
    assert_eq!(fs::read(counter).unwrap(), b"x");
}

#[test]
fn v05_environment_change_invalidates_reuse() {
    let task = task("v05");
    let counter = task.root.join("count");
    let script = task.root.join("ok");
    exe(&script, "#!/bin/sh\nprintf x >> \"$1\"\nexit 0\n");
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"ok","kind":"external","program":"{}","args":["{}"],"cwd":".","timeout_seconds":5,"environment":["JACU_ACC_ENV"]}}],"delivery":{{"required_check_ids":["ok"]}}}}"#,
            script.display(),
            counter.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("ok", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let session = prepared["session_id"].as_str().unwrap().to_string();
    let verify = |value: &str| {
        Command::new(bin())
            .args([
                "verify",
                "--repo",
                task.repo.to_str().unwrap(),
                "--session",
                &session,
                "--checkpoint",
                "delivery",
            ])
            .env("JACU_HOME", &task.home)
            .env("JACU_ACC_ENV", value)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };
    assert!(verify("one").status.success());
    assert!(verify("two").status.success());
    assert_eq!(fs::read(counter).unwrap(), b"xx");
    let report = String::from_utf8_lossy(&verify("two").stdout).into_owned();
    assert!(!report.contains("secret-one") && !report.contains("secret-two"));
}

#[test]
fn v06_untracked_file_changes_the_candidate() {
    let task = task("v06");
    let counter = task.root.join("count");
    let script = task.root.join("ok");
    exe(&script, "#!/bin/sh\nprintf x >> \"$1\"\nexit 0\n");
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"ok","kind":"external","program":"{}","args":["{}"],"cwd":".","timeout_seconds":5}}],"delivery":{{"required_check_ids":["ok"]}}}}"#,
            script.display(),
            counter.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("ok", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let session = prepared["session_id"].as_str().unwrap();
    let head = prepared["candidate"]["head"].clone();
    assert_eq!(task.verify(session, "delivery").0, 0);
    fs::write(task.repo.join("untracked.txt"), "new\n").unwrap();
    assert_eq!(task.verify(session, "delivery").0, 0);
    let (_, again) = task.verify(session, "delivery");
    assert_eq!(again["candidate"]["head"], head);
    assert_eq!(again["candidate"]["dirty"], true);
    assert_eq!(fs::read(counter).unwrap(), b"xx");
}

#[test]
fn v07_external_state_is_not_reused() {
    let task = task("v07");
    let counter = task.root.join("count");
    let script = task.root.join("ok");
    exe(&script, "#!/bin/sh\nprintf x >> \"$1\"\nexit 0\n");
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"ok","kind":"external","program":"{}","args":["{}"],"cwd":".","timeout_seconds":5,"external_state":true}}],"delivery":{{"required_check_ids":["ok"]}}}}"#,
            script.display(),
            counter.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("ok", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let session = prepared["session_id"].as_str().unwrap();
    assert_eq!(task.verify(session, "delivery").0, 0);
    assert_eq!(task.verify(session, "delivery").0, 0);
    assert_eq!(fs::read(counter).unwrap(), b"xx");
}

#[test]
fn v08_same_session_does_not_duplicate_or_kill_others() {
    let task = task("v08");
    let counter = task.root.join("count");
    let script = task.root.join("slow");
    exe(&script, "#!/bin/sh\nsleep 1\nprintf x >> \"$1\"\nexit 0\n");
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"slow","kind":"external","program":"{}","args":["{}"],"cwd":".","timeout_seconds":8}}],"delivery":{{"required_check_ids":["slow"]}}}}"#,
            script.display(),
            counter.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("slow", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let session = prepared["session_id"].as_str().unwrap().to_string();
    let mut sleeper = Command::new("/bin/sleep").arg("30").spawn().unwrap();
    let mut first = Command::new(bin())
        .args([
            "verify",
            "--repo",
            task.repo.to_str().unwrap(),
            "--session",
            &session,
            "--checkpoint",
            "delivery",
        ])
        .env("JACU_HOME", &task.home)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(200));
    let second = Command::new(bin())
        .args([
            "verify",
            "--repo",
            task.repo.to_str().unwrap(),
            "--session",
            &session,
            "--checkpoint",
            "delivery",
        ])
        .env("JACU_HOME", &task.home)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let first_status = first.wait().unwrap();
    assert!(first_status.success());
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stdout)
    );
    assert_eq!(fs::read(counter).unwrap(), b"x");
    assert!(sleeper.try_wait().unwrap().is_none());
    let _ = sleeper.kill();
}

#[test]
fn v09_changed_candidate_cannot_pass() {
    let task = task("v09");
    let script = task.root.join("edit");
    exe(
        &script,
        "#!/bin/sh\nprintf 'changed\\n' >> \"$1/README\"\nexit 0\n",
    );
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"edit","kind":"external","program":"{}","args":["{}"],"cwd":".","timeout_seconds":5}}],"delivery":{{"required_check_ids":["edit"]}}}}"#,
            script.display(),
            task.repo.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("edit", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let (code, value) = task.verify(prepared["session_id"].as_str().unwrap(), "delivery");
    assert_ne!(code, 0);
    assert_ne!(value["checks"][0]["state"], "passed", "{value}");
}

#[test]
fn v10_delivery_skips_a_passed_check() {
    let task = task("v10");
    let first = task.root.join("first");
    let second = task.root.join("second");
    let script = task.root.join("tick");
    exe(&script, "#!/bin/sh\nprintf x >> \"$1\"\nexit 0\n");
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"one","kind":"external","program":"{}","args":["{}"],"cwd":".","timeout_seconds":5}},{{"id":"two","kind":"external","program":"{}","args":["{}"],"cwd":".","timeout_seconds":5}}],"delivery":{{"required_check_ids":["one","two"]}}}}"#,
            script.display(),
            first.display(),
            script.display(),
            second.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(
        &contract_path,
        r#"{"schema_version":1,"outcomes":[{"id":"O1","statement":"Both.","implementation":"present","delivery":"in_target","evidence":["one","two"]}],"effort":{"uncertainty":0,"novelty":0,"coupling":0,"consequence":"ordinary"}}"#,
    )
    .unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let session = prepared["session_id"].as_str().unwrap();
    let _ = task.verify(session, "iteration");
    assert_eq!(task.verify(session, "delivery").0, 0);
    assert_eq!(fs::read(&first).unwrap(), b"x");
    assert_eq!(fs::read(&second).unwrap(), b"x");
}

#[test]
fn v11_smoke_output_is_in_the_receipt() {
    let task = task("v11");
    let script = task.root.join("smoke");
    exe(&script, "#!/bin/sh\nprintf 'native-smoke-ok\\n'\nexit 0\n");
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"smoke","kind":"external","program":"{}","args":[],"cwd":".","timeout_seconds":5}}],"delivery":{{"required_check_ids":["smoke"]}}}}"#,
            script.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("smoke", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let (_, value) = task.verify(prepared["session_id"].as_str().unwrap(), "delivery");
    assert!(value.to_string().contains("native-smoke-ok"), "{value}");
}

#[test]
fn c01_done_without_evidence_is_incomplete() {
    let task = task("c01");
    let contract_path = task.root.join("contract.json");
    fs::write(
        &contract_path,
        r#"{"schema_version":1,"outcomes":[{"id":"O1","statement":"done","implementation":"present","delivery":"in_target","evidence":[]}],"effort":{"uncertainty":0,"novelty":0,"coupling":0,"consequence":"ordinary"}}"#,
    )
    .unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let (code, value) = task.verify(prepared["session_id"].as_str().unwrap(), "delivery");
    assert_eq!(code, 3, "{value}");
    assert_eq!(value["outcome"], "incomplete");
}

#[test]
fn c02_omitted_outcome_stays_visible() {
    let task = task("c02");
    let contract_path = task.root.join("contract.json");
    fs::write(
        &contract_path,
        r#"{"schema_version":1,"outcomes":[{"id":"O1","statement":"Present.","implementation":"present","delivery":"in_target","evidence":[]},{"id":"O2","statement":"Still open.","implementation":"pending","delivery":"pending","evidence":[]}],"effort":{"uncertainty":0,"novelty":0,"coupling":0,"consequence":"ordinary"}}"#,
    )
    .unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let text = prepared.to_string();
    assert!(text.contains("O2"));
    assert!(prepared["pending"].to_string().contains("O2"));
}

#[test]
fn c03_weakened_assertion_cannot_close() {
    let task = task("c03");
    let script = task.root.join("unit");
    exe(
        &script,
        "#!/bin/sh\nprintf 'test result: ok. 1 passed\\n'\nexit 0\n",
    );
    let body = |minimum: u32| {
        format!(
            r#"{{"schema_version":1,"checks":[{{"id":"unit","kind":"cargo-test","program":"{}","args":[],"cwd":".","timeout_seconds":5,"minimum_tests":{minimum}}}],"delivery":{{"required_check_ids":["unit"]}}}}"#,
            script.display()
        )
    };
    policy(&task.repo, &body(2));
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("unit", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    policy(&task.repo, &body(1));
    let (code, value) = task.verify(prepared["session_id"].as_str().unwrap(), "delivery");
    assert_eq!(code, 3, "{value}");
    assert!(value["pending"].to_string().contains("weakened"));
}

#[test]
fn c04_unverified_or_unintegrated_is_not_complete() {
    let task = task("c04");
    let script = task.root.join("ok");
    exe(&script, "#!/bin/sh\nexit 0\n");
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"ok","kind":"external","program":"{}","args":[],"cwd":".","timeout_seconds":5}}],"delivery":{{"required_check_ids":["ok"]}}}}"#,
            script.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("ok", "present", "pending")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let (code, value) = task.verify(prepared["session_id"].as_str().unwrap(), "delivery");
    assert_eq!(code, 3, "{value}");
    assert_ne!(value["outcome"], "complete");
}

#[test]
fn c05_external_blocker_asks_nothing() {
    ni07_external_denial_is_not_a_permission_rewrite();
}

#[test]
fn c06_report_names_target_and_outcomes() {
    let task = task("c06");
    let script = task.root.join("ok");
    exe(&script, "#!/bin/sh\nexit 0\n");
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"ok","kind":"external","program":"{}","args":[],"cwd":".","timeout_seconds":5}}],"delivery":{{"required_check_ids":["ok"]}}}}"#,
            script.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("ok", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let session = prepared["session_id"].as_str().unwrap();
    assert_eq!(task.verify(session, "delivery").0, 0);
    let (_, report) = run(
        &task.home,
        &[
            "report",
            "--repo",
            task.repo.to_str().unwrap(),
            "--session",
            session,
        ],
    );
    assert_eq!(report["outcome"], "complete");
    assert_eq!(report["outcomes"][0]["id"], "O1");
    assert!(report["candidate"]["hash"].as_str().unwrap().len() > 16);
    assert!(report["candidate"]["repo"]
        .as_str()
        .unwrap()
        .contains("repo"));
}

#[test]
fn g01_other_worktree_is_listed() {
    let task = task("g01");
    let other = task.root.join("other");
    git(
        &task.repo,
        &["worktree", "add", other.to_str().unwrap(), "HEAD"],
    );
    let (_, value) = task.prepare(&[]);
    assert!(
        value["inventory"]["worktrees"].as_array().unwrap().len() >= 2,
        "{value}"
    );
}

#[test]
fn g02_unrelated_dirty_file_is_preserved() {
    let task = task("g02");
    fs::write(task.repo.join("notes.txt"), "keep\n").unwrap();
    let script = task.root.join("ok");
    exe(&script, "#!/bin/sh\nexit 0\n");
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"ok","kind":"external","program":"{}","args":[],"cwd":".","timeout_seconds":5}}],"delivery":{{"required_check_ids":["ok"]}}}}"#,
            script.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("ok", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    assert_eq!(
        task.verify(prepared["session_id"].as_str().unwrap(), "delivery")
            .0,
        0
    );
    assert_eq!(
        fs::read_to_string(task.repo.join("notes.txt")).unwrap(),
        "keep\n"
    );
}

#[test]
fn g03_reverted_history_is_not_current() {
    let task = task("g03");
    fs::write(task.repo.join("feature.txt"), "on\n").unwrap();
    git(&task.repo, &["add", "feature.txt"]);
    git(&task.repo, &["commit", "-m", "add"]);
    git(&task.repo, &["rm", "feature.txt"]);
    git(&task.repo, &["commit", "-m", "revert"]);
    let (_, value) = task.prepare(&[]);
    assert!(!task.repo.join("feature.txt").exists());
    assert_eq!(value["candidate"]["head"].as_str().unwrap().len(), 40);
    assert_ne!(value["outcome"], "complete");
}

#[test]
fn g04_delivery_follows_the_integrated_commit() {
    let task = task("g04");
    let counter = task.root.join("count");
    let script = task.root.join("ok");
    exe(&script, "#!/bin/sh\nprintf x >> \"$1\"\nexit 0\n");
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"ok","kind":"external","program":"{}","args":["{}"],"cwd":".","timeout_seconds":5}}],"delivery":{{"required_check_ids":["ok"]}}}}"#,
            script.display(),
            counter.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("ok", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let session = prepared["session_id"].as_str().unwrap();
    assert_eq!(task.verify(session, "delivery").0, 0);
    fs::write(task.repo.join("more.txt"), "integrated\n").unwrap();
    git(&task.repo, &["add", "more.txt"]);
    git(&task.repo, &["commit", "-m", "integrate"]);
    assert_eq!(task.verify(session, "delivery").0, 0);
    assert_eq!(fs::read(counter).unwrap(), b"xx");
}

#[test]
fn g05_missing_worktree_and_upstream_are_reported() {
    let task = task("g05");
    let other = task.root.join("gone");
    git(
        &task.repo,
        &["worktree", "add", other.to_str().unwrap(), "HEAD"],
    );
    fs::remove_dir_all(&other).unwrap();
    git(
        &task.repo,
        &[
            "remote",
            "add",
            "origin",
            "https://example.invalid/jacu.git",
        ],
    );
    let (_, value) = task.prepare(&[]);
    let notes = value["inventory"]["notes"].to_string();
    assert!(notes.contains("missing"), "{value}");
    assert!(notes.contains("upstream"), "{value}");
}

#[test]
fn g06_awkward_names_do_not_become_options() {
    let task = task("g06");
    fs::write(task.repo.join("-rf"), "safe\n").unwrap();
    fs::write(task.repo.join("café.txt"), "safe\n").unwrap();
    std::os::unix::fs::symlink("README", task.repo.join("link-readme")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let path = task.repo.join(std::ffi::OsStr::from_bytes(b"odd\nname"));
        fs::write(&path, "safe\n").unwrap();
    }
    git(&task.repo, &["add", "-A"]);
    git(&task.repo, &["commit", "-m", "names"]);
    let (code, value) = task.prepare(&[]);
    assert_eq!(code, 0, "{value}");
    assert_eq!(fs::read_to_string(task.repo.join("-rf")).unwrap(), "safe\n");
}

#[test]
fn e01_mechanical_task_is_focused() {
    let task = task("e01");
    let contract_path = task.root.join("contract.json");
    fs::write(
        &contract_path,
        r#"{"schema_version":1,"outcomes":[{"id":"O1","statement":"Rename nothing.","implementation":"pending","delivery":"pending","evidence":[]}],"effort":{"uncertainty":0,"novelty":0,"coupling":0,"consequence":"ordinary"}}"#,
    )
    .unwrap();
    let (_, value) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    assert_eq!(value["effort"]["recommended_effort"], "focused");
    assert_eq!(value["effort"]["applied_effort"], Value::Null);
}

#[test]
fn e02_critical_consequence_floors_to_deep() {
    let task = task("e02");
    let contract_path = task.root.join("contract.json");
    fs::write(
        &contract_path,
        r#"{"schema_version":1,"outcomes":[{"id":"O1","statement":"One line.","implementation":"pending","delivery":"pending","evidence":[]}],"effort":{"uncertainty":0,"novelty":0,"coupling":0,"consequence":"critical"}}"#,
    )
    .unwrap();
    let (_, value) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    assert_eq!(value["effort"]["recommended_effort"], "deep");
}

#[test]
fn e03_effort_stays_advisory() {
    let task = task("e03");
    let (_, value) = task.prepare(&[]);
    let (_, caps) = run(&task.home, &["capabilities"]);
    assert_eq!(caps["effort_application"], "advisory_only");
    assert!(value.get("effort").is_none() || value["effort"].is_null());
}

#[test]
fn e04_new_scores_can_escalate_without_a_subagent() {
    let task = task("e04");
    let low = task.root.join("low.json");
    let high = task.root.join("high.json");
    fs::write(&low, r#"{"schema_version":1,"outcomes":[{"id":"O1","statement":"Work.","implementation":"pending","delivery":"pending","evidence":[]}],"effort":{"uncertainty":0,"novelty":0,"coupling":0,"consequence":"ordinary"}}"#).unwrap();
    fs::write(&high, r#"{"schema_version":1,"outcomes":[{"id":"O1","statement":"Work.","implementation":"pending","delivery":"pending","evidence":[]}],"effort":{"uncertainty":2,"novelty":2,"coupling":2,"consequence":"ordinary"}}"#).unwrap();
    let (_, first) = task.prepare(&["--contract-file", low.to_str().unwrap()]);
    let session = first["session_id"].as_str().unwrap();
    let (_, second) = task.prepare(&[
        "--session",
        session,
        "--contract-file",
        high.to_str().unwrap(),
    ]);
    assert_eq!(first["effort"]["recommended_effort"], "focused");
    assert_eq!(second["effort"]["recommended_effort"], "deep");
    assert!(!second.to_string().contains("subagent"));
}

#[test]
fn sem01_missing_jev_does_not_block_or_call_out() {
    let task = task("sem01");
    let script = task.root.join("ok");
    exe(&script, "#!/bin/sh\nexit 0\n");
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"ok","kind":"external","program":"{}","args":[],"cwd":".","timeout_seconds":5}}],"delivery":{{"required_check_ids":["ok"]}}}}"#,
            script.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("ok", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let (code, value) = task.verify(prepared["session_id"].as_str().unwrap(), "delivery");
    assert_eq!(code, 0, "{value}");
    assert!(!value.to_string().contains("api.openai.com"));
    assert!(value["usage"]["cost"].is_null());
}

#[test]
fn sem02_malformed_semantic_answer_cannot_pass() {
    let task = task("sem02");
    let script = task.root.join("jev");
    exe(
        &script,
        "#!/bin/sh\nprintf '%s\\n' '{\"score\":null}'\nexit 0\n",
    );
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"meaning","kind":"semantic","program":"{}","args":[],"cwd":".","timeout_seconds":5}}],"delivery":{{"required_check_ids":["meaning"]}}}}"#,
            script.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("meaning", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let (code, value) = task.verify(prepared["session_id"].as_str().unwrap(), "delivery");
    assert_ne!(code, 0);
    assert_ne!(value["checks"][0]["state"], "passed", "{value}");
}

#[test]
fn sem03_semantic_pass_does_not_override_failure() {
    let task = task("sem03");
    let bad = task.root.join("bad");
    let script = task.root.join("jev");
    exe(&bad, "#!/bin/sh\nexit 1\n");
    exe(&script, "#!/bin/sh\nprintf '%s\\n' '{\"request_sha256\":\"later\",\"evidence_id\":\"meaning\",\"score\":1}'\nexit 0\n");
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"unit","kind":"external","program":"{}","args":[],"cwd":".","timeout_seconds":5}},{{"id":"meaning","kind":"semantic","program":"{}","args":[],"cwd":".","timeout_seconds":5}}],"delivery":{{"required_check_ids":["unit","meaning"]}}}}"#,
            bad.display(),
            script.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(
        &contract_path,
        r#"{"schema_version":1,"outcomes":[{"id":"O1","statement":"Both.","implementation":"present","delivery":"in_target","evidence":["unit","meaning"]}],"effort":{"uncertainty":0,"novelty":0,"coupling":0,"consequence":"ordinary"}}"#,
    )
    .unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let hash = prepared["request_sha256"].as_str().unwrap();
    exe(
        &script,
        &format!("#!/bin/sh\nprintf '%s\\n' '{{\"request_sha256\":\"{hash}\",\"evidence_id\":\"meaning\",\"score\":1}}'\nexit 0\n"),
    );
    let (code, value) = task.verify(prepared["session_id"].as_str().unwrap(), "delivery");
    assert_eq!(code, 2, "{value}");
    assert_eq!(value["outcome"], "failed");
}

#[test]
fn sem04_semantic_identity_must_match() {
    let task = task("sem04");
    let script = task.root.join("jev");
    exe(&script, "#!/bin/sh\nprintf '%s\\n' '{\"request_sha256\":\"wrong\",\"evidence_id\":\"meaning\",\"score\":1}'\nexit 0\n");
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"meaning","kind":"semantic","program":"{}","args":[],"cwd":".","timeout_seconds":5}}],"delivery":{{"required_check_ids":["meaning"]}}}}"#,
            script.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("meaning", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let (code, value) = task.verify(prepared["session_id"].as_str().unwrap(), "delivery");
    assert_ne!(code, 0);
    assert_eq!(value["checks"][0]["state"], "failed", "{value}");
}

#[test]
fn ct01_ambiguous_context_id_is_rejected() {
    let task = task("ct01");
    fs::write(task.repo.join("store.rs"), "fn save() {}\n").unwrap();
    let contract_path = task.root.join("contract.json");
    fs::write(
        &contract_path,
        r#"{"schema_version":1,"outcomes":[{"id":"O1","statement":"Save.","implementation":"pending","delivery":"pending","evidence":[]}],"context":[{"id":"f17","path":"store.rs"}],"effort":{"uncertainty":0,"novelty":0,"coupling":0,"consequence":"ordinary"}}"#,
    )
    .unwrap();
    let (code, value) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    assert_eq!(code, 4, "{value}");
    assert!(value["error"].as_str().unwrap().contains("f17"));
}

#[test]
fn ct02_source_names_stay_intact() {
    let task = task("ct02");
    fs::write(task.repo.join("settings_store.rs"), "fn save() {}\n").unwrap();
    git(&task.repo, &["add", "settings_store.rs"]);
    git(&task.repo, &["commit", "-m", "name"]);
    let (_, value) = task.prepare(&[]);
    assert_eq!(value["outcome"], "ready");
    assert_eq!(
        fs::read_to_string(task.repo.join("settings_store.rs")).unwrap(),
        "fn save() {}\n"
    );
}

#[test]
fn p01_package_exposes_skill_and_commands() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let skill = fs::read_to_string(manifest.join("skills/jacu-fast/SKILL.md")).unwrap();
    assert!(skill.contains("name: jacu-fast"));
    assert!(skill.contains("bin/jacu"));
    let (_, caps) = run(&temp("p01"), &["capabilities"]);
    assert_eq!(caps["compiles_on_missing_binary"], false);
    assert!(caps["commands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "prepare"));
}

#[test]
fn p02_missing_program_does_not_compile_or_pass() {
    let task = task("p02");
    policy(
        &task.repo,
        r#"{"schema_version":1,"checks":[{"id":"missing","kind":"external","program":"./missing-binary","args":[],"cwd":".","timeout_seconds":5}],"delivery":{"required_check_ids":["missing"]}}"#,
    );
    let (code, value) = task.prepare(&[]);
    assert_eq!(code, 4, "{value}");
    assert_ne!(value["outcome"], "complete");
    assert!(!value.to_string().contains("Compiling"));
}

#[test]
fn p03_hook_does_not_run_checks() {
    let task = task("p03");
    let counter = task.root.join("count");
    let script = task.root.join("tick");
    exe(&script, "#!/bin/sh\nprintf x >> \"$1\"\nexit 0\n");
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"tick","kind":"external","program":"{}","args":["{}"],"cwd":".","timeout_seconds":5}}],"delivery":{{"required_check_ids":["tick"]}}}}"#,
            script.display(),
            counter.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("tick", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let (_, value) = run(
        &task.home,
        &[
            "verify",
            "--hook",
            "claude",
            "--repo",
            task.repo.to_str().unwrap(),
            "--session",
            prepared["session_id"].as_str().unwrap(),
        ],
    );
    assert_ne!(value["outcome"], "complete");
    assert!(!counter.exists());
}

#[test]
fn p04_hook_recursion_stops() {
    let task = task("p04");
    let (_, prepared) = task.prepare(&["--audit"]);
    let output = Command::new(bin())
        .args([
            "verify",
            "--hook",
            "claude",
            "--repo",
            task.repo.to_str().unwrap(),
            "--session",
            prepared["session_id"].as_str().unwrap(),
        ])
        .env("JACU_HOME", &task.home)
        .env("JACU_HOOK", "1")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["terminal"], true);
    assert_eq!(value["next_actions"].as_array().unwrap().len(), 0);
}

#[test]
fn p05_session_pin_and_project_files_survive() {
    let task = task("p05");
    fs::write(task.repo.join("keep-me.txt"), "keep\n").unwrap();
    let (_, prepared) = task.prepare(&[]);
    let session = prepared["session_id"].as_str().unwrap();
    let path = task.home.join("repos");
    let mut found = None;
    for entry in walk(&path) {
        if entry.ends_with(format!("{session}.json")) {
            found = Some(entry);
        }
    }
    let file = found.expect("session file");
    let mut value: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    value["binary_version"] = Value::String("9.9.9".into());
    fs::write(&file, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    let (code, report) = run(
        &task.home,
        &[
            "report",
            "--repo",
            task.repo.to_str().unwrap(),
            "--session",
            session,
        ],
    );
    assert_eq!(code, 4, "{report}");
    let _ = run(
        &task.home,
        &["clean", "--repo", task.repo.to_str().unwrap()],
    );
    assert_eq!(
        fs::read_to_string(task.repo.join("keep-me.txt")).unwrap(),
        "keep\n"
    );
    assert_eq!(
        fs::read_to_string(task.repo.join("README")).unwrap(),
        "synthetic\n"
    );
}

#[test]
fn p06_documented_commands_match_capabilities() {
    let readme =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("README.md")).unwrap();
    for command in ["prepare", "verify", "report"] {
        assert!(readme.contains(command), "{command}");
    }
    let (_, caps) = run(&temp("p06"), &["capabilities"]);
    for command in ["prepare", "verify", "report", "clean", "capabilities"] {
        assert!(caps["commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == command));
    }
}

#[test]
fn p07_published_tree_has_no_private_material() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for relative in ["src", "tests", "skills", "README.md", "examples"] {
        for path in walk(&root.join(relative)) {
            if path.is_dir() {
                continue;
            }
            let text = fs::read_to_string(&path).unwrap_or_default();
            let markers = [
                format!("{}IA", "AK"),
                format!("BEGIN {}SSH", "OPEN"),
                format!("sk-{}", "live"),
                format!("PRIVATE-{}", "PILOT"),
            ];
            for marker in markers {
                assert!(
                    !text.contains(&marker),
                    "{} contains a private marker",
                    path.display()
                );
            }
        }
    }
}

#[test]
fn h01_inventory_deletes_nothing() {
    let task = task("h01");
    fs::create_dir_all(task.repo.join("target")).unwrap();
    fs::write(task.repo.join("target/cache.bin"), "cache").unwrap();
    let other = task.root.join("other");
    git(
        &task.repo,
        &["worktree", "add", other.to_str().unwrap(), "HEAD"],
    );
    let (_, _) = task.prepare(&[]);
    assert_eq!(
        fs::read(task.repo.join("target/cache.bin")).unwrap(),
        b"cache"
    );
    assert!(other.join("README").exists());
}

#[test]
fn h02_clean_removes_only_owned_expired_diagnostics() {
    let task = task("h02");
    let (_, prepared) = task.prepare(&[]);
    let session = prepared["session_id"].as_str().unwrap();
    let mut owned = None;
    for path in walk(&task.home.join("diagnostics")) {
        if path.ends_with(format!("{session}.json")) {
            owned = Some(path);
        }
    }
    let diagnostic = owned.expect("diagnostic");
    let ago = SystemTime::now() - Duration::from_secs(10);
    let _ = diagnostic.set_modified(ago);
    fs::write(task.repo.join("keep.txt"), "keep").unwrap();
    let active = diagnostic.with_file_name("active-lock.json");
    fs::write(&active, "busy").unwrap();
    let (code, value) = Command::new(bin())
        .args(["clean", "--repo", task.repo.to_str().unwrap()])
        .env("JACU_HOME", &task.home)
        .env("JACU_DIAGNOSTIC_TTL_SECONDS", "1")
        .stdin(Stdio::null())
        .output()
        .map(|output| {
            let value = serde_json::from_slice::<Value>(&output.stdout).unwrap();
            (output.status.code().unwrap(), value)
        })
        .unwrap();
    assert_eq!(code, 0, "{value}");
    assert!(!diagnostic.exists());
    assert!(active.exists());
    assert_eq!(fs::read(task.repo.join("keep.txt")).unwrap(), b"keep");
    assert!(value["reclaimed_bytes"].as_u64().unwrap() > 0);
}

#[test]
fn h03_changed_diagnostic_and_untracked_work_stay() {
    let task = task("h03");
    let (_, prepared) = task.prepare(&[]);
    let session = prepared["session_id"].as_str().unwrap();
    let diagnostic = walk(&task.home.join("diagnostics"))
        .into_iter()
        .find(|path| path.ends_with(format!("{session}.json")))
        .unwrap();
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(&diagnostic)
        .unwrap();
    writeln!(file, "changed").unwrap();
    let _ = diagnostic.set_modified(SystemTime::now() - Duration::from_secs(10));
    fs::write(task.repo.join("untracked.txt"), "mine\n").unwrap();
    let _ = Command::new(bin())
        .args(["clean", "--repo", task.repo.to_str().unwrap()])
        .env("JACU_HOME", &task.home)
        .env("JACU_DIAGNOSTIC_TTL_SECONDS", "1")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(diagnostic.exists());
    assert_eq!(
        fs::read_to_string(task.repo.join("untracked.txt")).unwrap(),
        "mine\n"
    );
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&task.repo)
        .output()
        .unwrap();
    assert!(head.status.success());
}

#[test]
fn h04_disk_count_and_build_timing() {
    let task = task("h04");
    fs::write(task.repo.join("blob.bin"), vec![7_u8; 128]).unwrap();
    std::os::unix::fs::symlink("blob.bin", task.repo.join("blob-link")).unwrap();
    fs::create_dir_all(task.repo.join("sample/src")).unwrap();
    fs::write(
        task.repo.join("sample/Cargo.toml"),
        "[package]\nname = \"sample\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(task.repo.join("sample/src/main.rs"), "fn main() {}\n").unwrap();
    fs::write(task.repo.join(".gitignore"), "sample/target/\n").unwrap();
    let generated = Command::new("cargo")
        .args(["generate-lockfile", "--offline", "--manifest-path"])
        .arg(task.repo.join("sample/Cargo.toml"))
        .status()
        .unwrap();
    assert!(generated.success());
    policy(
        &task.repo,
        r#"{"schema_version":1,"checks":[{"id":"build","kind":"cargo-check","program":"cargo","args":["check","--offline","--manifest-path","sample/Cargo.toml","--quiet"],"cwd":".","timeout_seconds":180}],"delivery":{"required_check_ids":["build"]}}"#,
    );
    let contract_path = task.root.join("contract.json");
    fs::write(&contract_path, contract("build", "present", "in_target")).unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let logical = prepared["inventory"]["disk_logical_bytes"]
        .as_u64()
        .unwrap();
    let blob = fs::metadata(task.repo.join("blob.bin")).unwrap().len();
    assert!(
        logical < blob * 2 + 50_000,
        "logical {logical} double-counted blob {blob}"
    );
    let (code, value) = task.verify(prepared["session_id"].as_str().unwrap(), "delivery");
    assert_eq!(code, 0, "{value}");
    assert!(value["timings_ms"]["next_build"].as_u64().is_some());
}

#[test]
fn m01_unknown_usage_is_null() {
    let task = task("m01");
    let (_, value) = task.prepare(&[]);
    assert!(value["usage"]["input_tokens"].is_null());
    assert!(value["usage"]["cost"].is_null());
    assert!(value["timings_ms"].get("next_build").is_none());
}

#[test]
fn m02_failures_count_in_the_denominator() {
    ni06_unchanged_failure_is_terminal_to_report();
}

#[test]
fn m03_deferred_checks_are_not_passes() {
    let task = task("m03");
    let script = task.root.join("tick");
    exe(&script, "#!/bin/sh\nexit 0\n");
    policy(
        &task.repo,
        &format!(
            r#"{{"schema_version":1,"checks":[{{"id":"one","kind":"external","program":"{}","args":[],"cwd":".","timeout_seconds":5}},{{"id":"two","kind":"external","program":"{}","args":[],"cwd":".","timeout_seconds":5}}],"delivery":{{"required_check_ids":["one","two"]}}}}"#,
            script.display(),
            script.display()
        ),
    );
    let contract_path = task.root.join("contract.json");
    fs::write(
        &contract_path,
        r#"{"schema_version":1,"outcomes":[{"id":"O1","statement":"Both.","implementation":"present","delivery":"in_target","evidence":["one","two"]}],"effort":{"uncertainty":0,"novelty":0,"coupling":0,"consequence":"ordinary"}}"#,
    )
    .unwrap();
    let (_, prepared) = task.prepare(&["--contract-file", contract_path.to_str().unwrap()]);
    let (_, value) = task.verify(prepared["session_id"].as_str().unwrap(), "iteration");
    let states: Vec<_> = value["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["state"].as_str().unwrap().to_string())
        .collect();
    assert!(
        states.contains(&"deferred".to_string()) || states.contains(&"missing".to_string()),
        "{value}"
    );
    assert!(states.iter().any(|state| state != "passed"));
    assert_ne!(value["outcome"], "complete");
}

fn walk(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        if path.is_dir() {
            if let Ok(entries) = fs::read_dir(&path) {
                for entry in entries.flatten() {
                    stack.push(entry.path());
                }
            }
        }
        found.push(path);
    }
    found
}

trait Touch {
    fn set_modified(&self, time: SystemTime) -> std::io::Result<()>;
}

impl Touch for Path {
    fn set_modified(&self, time: SystemTime) -> std::io::Result<()> {
        let file = fs::File::options().write(true).open(self)?;
        file.set_modified(time)
    }
}
