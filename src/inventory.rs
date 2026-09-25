//! Content-addressed snapshots. Display strings are never used as path identity.
use crate::runner;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

pub(crate) struct Facts {
    pub root: PathBuf,
    pub git: bool,
    pub head: Option<String>,
    pub branch: Option<String>,
    pub dirty: bool,
    pub hash: String,
    pub worktrees: Vec<String>,
    pub notes: Vec<String>,
    pub logical_bytes: u64,
    pub allocated_bytes: u64,
}
pub(crate) fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let mut command = Command::new("git");
    command.arg("-C").arg(root).args(args);
    let result = runner::capture(command, Duration::from_secs(5), 8 * 1024 * 1024)?;
    if result.code != Some(0) || result.truncated || result.timed_out {
        return Err(format!(
            "git {} failed or exceeded its deadline: {}",
            args.join(" "),
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    Ok(result.stdout)
}
fn text(bytes: &[u8]) -> Result<String, String> {
    Ok(std::str::from_utf8(bytes)
        .map_err(|_| "Git returned a non-UTF-8 metadata field")?
        .trim_end_matches('\n')
        .to_string())
}
pub(crate) fn root(path: &Path) -> Result<PathBuf, String> {
    let canonical =
        fs::canonicalize(path).map_err(|e| format!("repository is not available: {e}"))?;
    if !canonical.is_dir() {
        return Err("repository path is not a directory".into());
    }
    match git(&canonical, &["rev-parse", "--show-toplevel"]) {
        Ok(out) => fs::canonicalize(text(&out)?).map_err(|e| e.to_string()),
        Err(e) => {
            if canonical.ancestors().any(|p| p.join(".git").exists()) {
                Err(e)
            } else {
                Ok(canonical)
            }
        }
    }
}
pub(crate) fn inspect(path: &Path, extra: &[String]) -> Result<Facts, String> {
    let started = Instant::now();
    let root = root(path)?;
    let has_git = git(&root, &["rev-parse", "--show-toplevel"]).is_ok();
    let head = git(&root, &["rev-parse", "--verify", "HEAD"])
        .ok()
        .and_then(|b| text(&b).ok());
    let branch = git(&root, &["symbolic-ref", "--quiet", "--short", "HEAD"])
        .ok()
        .and_then(|b| text(&b).ok());
    let status = if has_git {
        git(
            &root,
            &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
        )?
    } else {
        Vec::new()
    };
    let mut notes = Vec::new();
    if !has_git {
        notes.push("not a Git repository; snapshot includes all regular files".into());
    }
    if has_git && head.is_none() {
        notes.push("git HEAD is not available".into());
    }
    let worktrees = if has_git {
        git(&root, &["worktree", "list", "--porcelain", "-z"])?
            .split(|b| *b == 0)
            .filter_map(|f| f.strip_prefix(b"worktree "))
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    for w in &worktrees {
        if !Path::new(w).exists() {
            notes.push(format!("worktree is missing: {w}"));
        }
    }
    if has_git
        && !git(&root, &["remote"])?.is_empty()
        && git(&root, &["rev-parse", "--abbrev-ref", "@{upstream}"]).is_err()
    {
        notes.push("upstream ref is not available and is not assumed safe".into());
    }
    let mut snapshot = Snapshot {
        hash: Sha256::new(),
        start: started,
        files: 0,
        logical: 0,
        allocated: 0,
        inodes: BTreeSet::new(),
    };
    snapshot.field(b"jacu-content-snapshot-v2");
    snapshot.field(head.as_deref().unwrap_or("").as_bytes());
    snapshot.field(branch.as_deref().unwrap_or("").as_bytes());
    snapshot.tree(&root, &root, has_git, 0)?;
    let mut extra_sorted = extra.to_vec();
    extra_sorted.sort();
    extra_sorted.dedup();
    for input in extra_sorted {
        let path = crate::within(&root, &input)?;
        snapshot.field(b"declared-input");
        snapshot.field(input.as_bytes());
        if path.is_dir() {
            snapshot.directory(&root, &path, 0)?;
        } else {
            snapshot.file(&root, &path)?;
        }
    }
    if has_git {
        if status
            != git(
                &root,
                &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
            )?
            || head
                != git(&root, &["rev-parse", "--verify", "HEAD"])
                    .ok()
                    .and_then(|b| text(&b).ok())
        {
            return Err("repository changed while its snapshot was being collected".into());
        }
    }
    let hash = format!("{:x}", snapshot.hash.finalize());
    Ok(Facts {
        root,
        git: has_git,
        head,
        branch,
        dirty: !status.is_empty(),
        hash,
        worktrees,
        notes,
        logical_bytes: snapshot.logical,
        allocated_bytes: snapshot.allocated,
    })
}
struct Snapshot {
    hash: Sha256,
    start: Instant,
    files: usize,
    logical: u64,
    allocated: u64,
    inodes: BTreeSet<(u64, u64)>,
}
impl Snapshot {
    fn field(&mut self, b: &[u8]) {
        self.hash.update((b.len() as u64).to_le_bytes());
        self.hash.update(b);
    }
    fn deadline(&self) -> Result<(), String> {
        if self.start.elapsed() > Duration::from_secs(15) || self.files > 100000 {
            Err("snapshot exceeded its time or file-count bound; evidence is unavailable".into())
        } else {
            Ok(())
        }
    }
    fn tree(
        &mut self,
        boundary: &Path,
        dir: &Path,
        is_git: bool,
        depth: usize,
    ) -> Result<(), String> {
        self.deadline()?;
        if depth > 16 {
            return Err("nested repository limit exceeded".into());
        }
        if !is_git {
            return self.directory(boundary, dir, depth);
        }
        let files = git(
            dir,
            &[
                "ls-files",
                "--cached",
                "--others",
                "--exclude-standard",
                "-z",
            ],
        )?;
        let index = git(dir, &["ls-files", "--stage", "-z"])?;
        self.field(&index);
        let paths: BTreeSet<Vec<u8>> = files
            .split(|b| *b == 0)
            .filter(|b| !b.is_empty())
            .map(|b| b.to_vec())
            .collect();
        for bytes in &paths {
            self.deadline()?;
            let relative = path_bytes(bytes)?;
            if relative.is_absolute()
                || relative
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                return Err("Git path escapes the repository".into());
            }
            self.field(bytes);
            let path = dir.join(&relative);
            match fs::symlink_metadata(&path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => self.field(b"deleted"),
                Err(e) => return Err(e.to_string()),
                Ok(m) if m.is_dir() => {
                    self.field(b"submodule-or-nested-repository");
                    // A missing/uninitialized submodule cannot be treated as empty input.
                    if !path.join(".git").exists() {
                        return Err(format!("submodule {} is not initialized", path.display()));
                    }
                    self.field(&git(&path, &["rev-parse", "HEAD"])?);
                    self.tree(boundary, &path, true, depth + 1)?;
                }
                Ok(_) => self.file(boundary, &path)?,
            }
        }
        if files
            != git(
                dir,
                &[
                    "ls-files",
                    "--cached",
                    "--others",
                    "--exclude-standard",
                    "-z",
                ],
            )?
            || index != git(dir, &["ls-files", "--stage", "-z"])?
        {
            return Err("file inventory changed during snapshot".into());
        }
        Ok(())
    }
    fn directory(&mut self, boundary: &Path, dir: &Path, depth: usize) -> Result<(), String> {
        self.deadline()?;
        if depth > 64 {
            return Err("directory nesting limit exceeded".into());
        }
        let mut entries = fs::read_dir(dir)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            if e.file_name() == ".git" {
                continue;
            }
            let path = e.path();
            self.field(&os_bytes(
                path.strip_prefix(boundary)
                    .map_err(|e| e.to_string())?
                    .as_os_str(),
            ));
            if e.file_type().map_err(|e| e.to_string())?.is_dir() {
                self.directory(boundary, &path, depth + 1)?;
            } else {
                self.file(boundary, &path)?;
            }
        }
        Ok(())
    }
    fn file(&mut self, boundary: &Path, path: &Path) -> Result<(), String> {
        self.deadline()?;
        self.files += 1;
        let before = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if before.file_type().is_symlink() {
            let link_before = fs::read_link(path).map_err(|e| e.to_string())?;
            self.field(b"symlink");
            self.field(&os_bytes(link_before.as_os_str()));
            let target =
                fs::canonicalize(path).map_err(|e| format!("symlink target unavailable: {e}"))?;
            if !target.starts_with(boundary) || !target.is_file() {
                return Err(
                    "symlink input must resolve to a regular file inside the repository".into(),
                );
            }
            // Hash the dereferenced input too, including ignored targets.
            self.file(boundary, &target)?;
            if fs::read_link(path).map_err(|e| e.to_string())? != link_before {
                return Err("symlink changed during snapshot".into());
            }
            return Ok(());
        }
        if !before.is_file() {
            return Err("special filesystem objects are not valid snapshot inputs".into());
        }
        self.field(b"file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            self.field(&(before.mode() & 0o777).to_le_bytes());
            if self.inodes.insert((before.dev(), before.ino())) {
                self.logical += before.len();
                self.allocated += before.blocks() * 512;
            }
        }
        #[cfg(not(unix))]
        {
            self.logical += before.len();
            self.allocated += before.len();
        }
        let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
        let mut h = Sha256::new();
        let mut buf = [0u8; 65536];
        loop {
            self.deadline()?;
            let n = file.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            h.update(&buf[..n]);
        }
        let after = fs::metadata(path).map_err(|e| e.to_string())?;
        if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
            return Err("file changed during snapshot".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if before.ino() != after.ino()
                || before.dev() != after.dev()
                || before.mode() != after.mode()
            {
                return Err("file identity changed during snapshot".into());
            }
        }
        self.field(&h.finalize());
        Ok(())
    }
}
fn os_bytes(s: &std::ffi::OsStr) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        s.as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        s.to_string_lossy().as_bytes().to_vec()
    }
}
fn path_bytes(b: &[u8]) -> Result<PathBuf, String> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Ok(PathBuf::from(OsString::from_vec(b.to_vec())))
    }
    #[cfg(not(unix))]
    {
        Ok(PathBuf::from(std::str::from_utf8(b).map_err(|_| {
            "non-UTF-8 path is unsupported on this platform"
        })?))
    }
}
