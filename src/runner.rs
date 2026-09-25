//! Bounded, non-interactive process execution and evidence classification.
use crate::{
    argv_of as argv, effective_minimum, inventory, resolve_cwd as within, sha256_hex as digest,
    CheckSpec as Check, Receipt,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

pub(crate) struct Captured {
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub truncated: bool,
    pub timed_out: bool,
}
pub(crate) fn capture(
    mut command: Command,
    timeout: Duration,
    cap: usize,
) -> Result<Captured, String> {
    // Shell launchers repair PWD but direct hook processes do not. Normalize
    // the actual child environment, not just its fingerprint. The canonical
    // execution directory is already part of the repository/check identity.
    let cwd = match command.get_current_dir() {
        Some(path) => {
            fs::canonicalize(path).map_err(|e| format!("cannot resolve command cwd: {e}"))?
        }
        None => std::env::current_dir()
            .and_then(fs::canonicalize)
            .map_err(|e| format!("cannot resolve current directory: {e}"))?,
    };
    command.env("PWD", cwd);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .env("GIT_PAGER", "cat")
        .env("PAGER", "cat")
        .env("GIT_EDITOR", "true")
        .env("EDITOR", "true")
        .env("VISUAL", "true")
        .env("CI", "true")
        .env_remove("_")
        .env_remove("SHLVL");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let start = Instant::now();
    let mut child = command
        .spawn()
        .map_err(|e| format!("cannot start command: {e}"))?;
    let out = child.stdout.take().ok_or("stdout pipe unavailable")?;
    let err = child.stderr.take().ok_or("stderr pipe unavailable")?;
    let (tx, rx) = mpsc::channel();
    let tx2 = tx.clone();
    thread::spawn(move || {
        let _ = tx.send((true, read_capped(out, cap)));
    });
    thread::spawn(move || {
        let _ = tx2.send((false, read_capped(err, cap)));
    });
    let mut status = None;
    let mut ended = false;
    let mut timed_out = false;
    let mut stdout = None;
    let mut stderr = None;
    loop {
        while let Ok((is_out, stream)) = rx.try_recv() {
            if is_out {
                stdout = Some(stream)
            } else {
                stderr = Some(stream)
            }
        }
        if !ended {
            match child.try_wait() {
                Ok(Some(s)) => {
                    status = s.code();
                    ended = true;
                }
                Ok(None) => {}
                Err(e) => {
                    kill(&mut child);
                    let _ = child.wait();
                    return Err(e.to_string());
                }
            }
        }
        if ended && stdout.is_some() && stderr.is_some() {
            break;
        }
        if start.elapsed() >= timeout {
            timed_out = true;
            kill(&mut child);
            let _ = child.wait();
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    // Readers cannot extend the process deadline indefinitely when a descendant owns a pipe.
    let drain = Instant::now();
    while (stdout.is_none() || stderr.is_none()) && drain.elapsed() < Duration::from_millis(200) {
        if let Ok((is_out, stream)) = rx.recv_timeout(Duration::from_millis(10)) {
            if is_out {
                stdout = Some(stream)
            } else {
                stderr = Some(stream)
            }
        }
    }
    let (stdout, ot) = stdout.unwrap_or((Vec::new(), true));
    let (stderr, et) = stderr.unwrap_or((Vec::new(), true));
    Ok(Captured {
        code: status,
        stdout,
        stderr,
        truncated: ot || et,
        timed_out,
    })
}
fn read_capped(mut pipe: impl Read, cap: usize) -> (Vec<u8>, bool) {
    let mut bytes = Vec::new();
    let mut buf = [0u8; 8192];
    let mut truncated = false;
    loop {
        match pipe.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let take = n.min(cap.saturating_sub(bytes.len()));
                bytes.extend_from_slice(&buf[..take]);
                truncated |= take < n;
            }
            Err(_) => {
                truncated = true;
                break;
            }
        }
    }
    (bytes, truncated)
}
fn kill(child: &mut Child) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.kill();
}
fn resolve(root: &Path, c: &Check) -> Result<PathBuf, String> {
    let cwd = within(root, &c.cwd)?;
    let path = Path::new(&c.program);
    if path.is_absolute() || c.program.contains('/') || c.program.contains('\\') {
        return fs::canonicalize(if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        })
        .map_err(|e| e.to_string());
    }
    for dir in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        let p = dir.join(&c.program);
        if p.is_file() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if fs::metadata(&p)
                    .map_err(|e| e.to_string())?
                    .permissions()
                    .mode()
                    & 0o111
                    == 0
                {
                    continue;
                }
            }
            return fs::canonicalize(p).map_err(|e| e.to_string());
        }
    }
    Err(format!("program {} is not available on PATH", c.program))
}
fn file_hash(path: &Path) -> Result<String, String> {
    let start = Instant::now();
    let mut f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut h = Sha256::new();
    let mut b = [0u8; 65536];
    loop {
        if start.elapsed() > Duration::from_secs(3) {
            return Err("executable fingerprint timed out".into());
        }
        let n = f.read(&mut b).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
pub(crate) fn program_hash(root: &Path, c: &Check) -> Result<String, String> {
    let path = resolve(root, c)?;
    let mut identity = format!("{}:{}", path.display(), file_hash(&path)?);
    // Version output captures rustup-selected toolchains, not only the rustup shim.
    let name = Path::new(&c.program)
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("");
    if matches!(
        name,
        "cargo" | "rustc" | "node" | "npm" | "pnpm" | "yarn" | "bun" | "go" | "swift" | "python3"
    ) {
        let mut cmd = Command::new(&c.program);
        cmd.current_dir(within(root, &c.cwd)?).arg(if name == "go" {
            "version"
        } else {
            "--version"
        });
        let v = capture(cmd, Duration::from_secs(3), 65536)?;
        if v.code != Some(0) || v.timed_out || v.truncated {
            return Err("toolchain version is unavailable".into());
        }
        identity.push_str(&digest(&v.stdout));
        identity.push_str(&digest(&v.stderr));
    }
    Ok(digest(identity.as_bytes()))
}
pub(crate) fn environment_hash(_declared: &[String]) -> String {
    // Hash the complete inherited environment; never persist its values.
    let mut effective: std::collections::BTreeMap<_, _> = std::env::vars_os().collect();
    for (k, v) in [
        ("GIT_TERMINAL_PROMPT", "0"),
        ("GCM_INTERACTIVE", "never"),
        ("GIT_PAGER", "cat"),
        ("PAGER", "cat"),
        ("GIT_EDITOR", "true"),
        ("EDITOR", "true"),
        ("VISUAL", "true"),
        ("CI", "true"),
    ] {
        effective.insert(k.into(), v.into());
    }
    effective.remove(std::ffi::OsStr::new("_"));
    effective.remove(std::ffi::OsStr::new("SHLVL"));
    // capture replaces PWD with the canonical command cwd on every execution.
    // Inherited stale PWD is not an input; receipt cwd and repo bind the value.
    effective.remove(std::ffi::OsStr::new("PWD"));
    let env: Vec<_> = effective.into_iter().collect();
    let mut h = Sha256::new();
    for (k, v) in env {
        for value in [k, v] {
            #[cfg(unix)]
            let bytes = {
                use std::os::unix::ffi::OsStrExt;
                value.as_os_str().as_bytes().to_vec()
            };
            #[cfg(not(unix))]
            let bytes = value.to_string_lossy().as_bytes().to_vec();
            h.update((bytes.len() as u64).to_le_bytes());
            h.update(bytes);
        }
    }
    format!("{:x}", h.finalize())
}
pub(crate) fn execute(
    root: &Path,
    c: &Check,
    candidate: &str,
    policy: &str,
    request: &str,
    all_inputs: &[String],
) -> Result<Receipt, String> {
    let start = Instant::now();
    let before = program_hash(root, c).unwrap_or_else(|_| "unavailable".into());
    let environment = environment_hash(&c.environment);
    let mut cmd = Command::new(&c.program);
    cmd.current_dir(within(root, &c.cwd)?).args(&c.args);
    let result = capture(cmd, Duration::from_secs(c.timeout_seconds), 262144);
    let (mut state, mut detail, code, stdout, stderr, truncated, external) = match result {
        Err(e) => (
            "unknown".to_string(),
            e.clone(),
            None,
            Vec::new(),
            e.into_bytes(),
            false,
            true,
        ),
        Ok(r) => {
            let combined = format!(
                "{}\n{}",
                String::from_utf8_lossy(&r.stdout),
                String::from_utf8_lossy(&r.stderr)
            );
            let (state, detail) = if r.timed_out {
                ("unknown".into(), "check timed out".into())
            } else {
                classify(c, r.code, &combined, r.truncated, request)
            };
            (
                state,
                detail,
                r.code,
                r.stdout,
                r.stderr,
                r.truncated || r.timed_out,
                matches!(r.code, Some(126 | 127)),
            )
        }
    };
    if state == "passed" {
        // The caller also checks the entire policy input union after the set.
        match inventory::inspect(root, all_inputs) {
            Ok(after) if after.hash != candidate => {
                state = "unknown".into();
                detail = "candidate changed during the check".into();
            }
            Err(e) => {
                state = "unknown".into();
                detail = format!("candidate cannot be re-observed: {e}");
            }
            _ => {}
        }
        if before == "unavailable" || program_hash(root, c).ok().as_deref() != Some(before.as_str())
        {
            state = "unknown".into();
            detail = "executable/toolchain changed or could not be fingerprinted".into();
        }
    }
    Ok(Receipt {
        check_id: c.id.clone(),
        state,
        exit_code: code,
        candidate_hash: candidate.into(),
        argv: argv(c),
        cwd: c.cwd.clone(),
        policy_sha256: policy.into(),
        duration_ms: start.elapsed().as_millis(),
        stdout_sha256: digest(&stdout),
        stderr_sha256: digest(&stderr),
        stdout_excerpt: String::from_utf8_lossy(&stdout[..stdout.len().min(1200)]).into(),
        stderr_excerpt: String::from_utf8_lossy(&stderr[..stderr.len().min(1200)]).into(),
        truncated,
        detail,
        external,
        env_sha256: environment,
        kind: c.kind.clone(),
        request_sha256: request.into(),
        program_sha256: before,
    })
}
fn classify(
    c: &Check,
    code: Option<i32>,
    text: &str,
    truncated: bool,
    request: &str,
) -> (String, String) {
    if truncated {
        return ("unknown".into(), "check output was truncated".into());
    }
    if code != Some(0) {
        return (
            "failed".into(),
            format!(
                "check exited with {}",
                code.map(|x| x.to_string())
                    .unwrap_or_else(|| "signal".into())
            ),
        );
    }
    let count = passed_count(text, &c.kind);
    let zero = text.contains("running 0 tests")
        || text.contains("No tests found")
        || text.contains("no tests ran")
        || text.contains("[no test files]");
    if count == Some(0) || (zero && count.unwrap_or(0) == 0) {
        return ("failed".into(), "filter selected zero tests".into());
    }
    let min = effective_minimum(c);
    if min > 0 {
        match count{Some(n)if n>=min=>{},Some(n)=>return("failed".into(),format!("passed {n} tests; minimum is {min}")),None=>return("unknown".into(),"test count is not recognized; configure a supported test output instead of treating exit 0 as proof".into())}
    }
    if c.kind == "semantic" {
        let v: Value = match serde_json::from_str(text.trim()) {
            Ok(v) => v,
            Err(_) => return ("failed".into(), "semantic answer is not JSON".into()),
        };
        if v["request_sha256"].as_str() != Some(request)
            || v["evidence_id"].as_str() != Some(c.id.as_str())
        {
            return (
                "failed".into(),
                "semantic answer is not bound to this request and check".into(),
            );
        }
        return match v["score"].as_f64() {
            Some(n) if n.is_finite() && (0.0..=1.0).contains(&n) && n >= c.minimum_score => (
                "passed".into(),
                "semantic threshold passed; this does not waive behavioral checks".into(),
            ),
            _ => (
                "failed".into(),
                "semantic score is invalid or below the required threshold".into(),
            ),
        };
    }
    ("passed".into(), "check passed".into())
}
fn passed_count(text: &str, kind: &str) -> Option<u32> {
    if kind == "go-test" {
        let mut count = 0;
        let mut seen = false;
        for l in text.lines() {
            if let Ok(v) = serde_json::from_str::<Value>(l) {
                seen |= v.get("Action").is_some();
                if v["Action"] == "pass" && v["Test"].is_string() {
                    count += 1;
                }
            }
        }
        return seen.then_some(count);
    }
    let mut cargo = 0u32;
    let mut cargo_seen = false;
    let mut other = None;
    for line in text.lines() {
        if let Some(index) = line.find(" passed") {
            let prefix = line[..index].trim_end();
            let digits: String = prefix
                .chars()
                .rev()
                .take_while(|x| x.is_ascii_digit())
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            if let Ok(n) = digits.parse::<u32>() {
                if line.contains("test result:") {
                    cargo = cargo.saturating_add(n);
                    cargo_seen = true;
                } else {
                    other = Some(other.unwrap_or(0u32).max(n));
                }
            }
        }
        if let Some(t) = line.trim().strip_prefix("Executed ") {
            if line.contains("0 failures") {
                if let Some(n) = t
                    .split_whitespace()
                    .next()
                    .and_then(|x| x.parse::<u32>().ok())
                {
                    other = Some(other.unwrap_or(0u32).max(n));
                }
            }
        }
        if let Some(t) = line.split("Test run with ").nth(1) {
            if line.contains(" passed") {
                if let Some(n) = t
                    .split_whitespace()
                    .next()
                    .and_then(|x| x.parse::<u32>().ok())
                {
                    other = Some(other.unwrap_or(0u32).max(n));
                }
            }
        }
    }
    if cargo_seen {
        Some(cargo)
    } else {
        other
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cargo_aggregates_nonempty_suites_and_ignores_empty_doctests() {
        assert_eq!(passed_count("running 0 tests\ntest result: ok. 0 passed; 0 failed\ntest result: ok. 3 passed; 0 failed\ntest result: ok. 0 passed; 0 failed","cargo-test"),Some(3));
    }
    #[test]
    fn go_counts_test_events_not_package_success() {
        assert_eq!(
            passed_count(
                "{\"Action\":\"pass\",\"Package\":\"p\"}\n{\"Action\":\"pass\",\"Test\":\"TestX\"}",
                "go-test"
            ),
            Some(1)
        );
    }
    #[test]
    fn swift_aggregate_is_not_counted_twice() {
        assert_eq!(
            passed_count(
                "Executed 3 tests, with 0 failures\nExecuted 3 tests, with 0 failures",
                "swift-test"
            ),
            Some(3)
        );
    }
}
