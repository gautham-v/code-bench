//! Shared by the held-out context tests: temp repos and a `Context` built
//! through the public API only (`RepoVocab::build` on a real directory).
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use squawk_core::context::{self, Agent, Context, RepoVocab, Session};

pub fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=You",
            "-c",
            "user.email=you@example.com",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

pub fn git_out(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

pub fn write(dir: &Path, rel: &str, body: &str) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, body).unwrap();
}

/// A temp directory whose path has no symlinks in it (macOS hands out
/// /var/..., which is /private/var/...).
pub struct Dir {
    _tmp: tempfile::TempDir,
    pub path: PathBuf,
}

pub fn tempdir() -> Dir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().canonicalize().expect("canonical tempdir");
    Dir { _tmp: tmp, path }
}

/// A git repo under a fresh temp dir with these files, committed.
pub fn git_repo(files: &[(&str, &str)]) -> Dir {
    let dir = tempdir();
    let repo = dir.path.join("widget");
    fs::create_dir_all(&repo).unwrap();
    for (rel, body) in files {
        write(&repo, rel, body);
    }
    git(&repo, &["init", "-q"]);
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-q", "-m", "init"]);
    Dir {
        _tmp: dir._tmp,
        path: repo,
    }
}

pub const README: &str = "\
# Widget

Widget talks to Tidewell over GPUI. The engine runs locally. It is fast.

Open the app to try it. Run the tests before you push.
";

/// The repo most tests talk about.
pub const FILES: &[(&str, &str)] = &[
    (
        "Cargo.toml",
        "[package]\nname = \"widget-core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    ),
    ("README.md", README),
    ("CLAUDE.md", "Notes for the agent. Reports go to Brightwave.\n"),
    (".gitignore", "target/\n"),
    ("src/main.rs", "fn main() {}\n"),
    ("src/audio.rs", ""),
    ("src/status_item.rs", ""),
    ("src/context/mod.rs", ""),
    ("src/store/mod.rs", ""),
    ("crates/app/src/ui/popover.rs", ""),
    ("web/ContentView.tsx", ""),
    ("web/vite.config.ts", ""),
    ("scripts/build-release.sh", ""),
    ("config.json", "{}\n"),
    ("config.json.example", "{}\n"),
    ("cmd/run.go", "package main\n"),
];

pub fn context_for(cwd: &Path) -> Context {
    Context {
        session: Session {
            agent: Agent::Claude,
            pid: 1,
            cwd: cwd.to_path_buf(),
            tty: None,
        },
        vocab: Arc::new(RepoVocab::build(cwd)),
    }
}

/// The widget repo and a context for a session at its root.
pub fn widget() -> (Dir, Context) {
    let dir = git_repo(FILES);
    let ctx = context_for(&dir.path);
    (dir, ctx)
}

pub fn apply(ctx: &Context, text: &str) -> String {
    context::apply(text, ctx)
}
