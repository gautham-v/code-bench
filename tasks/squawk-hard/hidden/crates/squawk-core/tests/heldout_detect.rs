//! `detect`: the agent session under the front terminal.
//!
//! The process trees are real. A copy of this test binary named `claude` or
//! `codex` stands in for the agent (see `agent_stand_in`), started under a
//! `/bin/sh` that stands in for the terminal, so each test has a tree of its
//! own and nothing else on the machine matters. The stand-ins exit when the
//! test lets go of their stdin.

mod heldout_support;

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use heldout_support::{tempdir, write, Dir};
use squawk_core::context::{detect, Agent, FrontApp, Session};

/// Not a test of its own: what a stand-in process runs. It says who it is
/// and then waits for its stdin to close.
#[test]
#[ignore]
fn agent_stand_in() {
    let Some(pid_file) = std::env::var_os("HELDOUT_PID_FILE") else {
        return;
    };
    let pid_file = PathBuf::from(pid_file);
    let tmp = pid_file.with_extension("tmp");
    fs::write(&tmp, std::process::id().to_string()).unwrap();
    fs::rename(&tmp, &pid_file).unwrap();
    // An agent that runs another one under itself, in another directory.
    let child = std::env::var_os("HELDOUT_CHILD_BIN").map(|bin| {
        Command::new(bin)
            .args(["agent_stand_in", "--exact", "--ignored", "--nocapture"])
            .env_remove("HELDOUT_CHILD_BIN")
            .env("HELDOUT_PID_FILE", pid_file.with_extension("child"))
            .current_dir(std::env::var_os("HELDOUT_CHILD_CWD").unwrap())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("child stand-in starts")
    });
    let mut sink = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut sink);
    if let Some(mut child) = child {
        let _ = child.wait();
    }
}

/// Nothing here is timed: these are only how long a wait may go on before
/// it is called a failure. The stand-in is up within milliseconds, and a
/// working `detect` then finds it on the first try.
const START_WAIT: Duration = Duration::from_secs(60);
const DETECT_WAIT: Duration = Duration::from_secs(15);

fn wait_for<T>(what: &str, limit: Duration, mut ready: impl FnMut() -> Option<T>) -> T {
    let started = Instant::now();
    loop {
        if let Some(v) = ready() {
            return v;
        }
        assert!(started.elapsed() < limit, "gave up waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// A "terminal" (`/bin/sh`) with one process named `name` under it, either
/// directly or behind two more shells, running in `cwd`.
struct Tree {
    root: Child,
    pid_file: PathBuf,
    _bin: Dir,
}

impl Tree {
    fn start(name: &str, cwd: &Path, behind_shells: bool) -> Tree {
        Tree::start_with(name, cwd, behind_shells, None)
    }

    /// `inner`: a second stand-in (name, cwd) that the first one runs
    /// under itself.
    fn start_with(
        name: &str,
        cwd: &Path,
        behind_shells: bool,
        inner: Option<(&str, &Path)>,
    ) -> Tree {
        let bin = tempdir();
        let exe = bin.path.join(name);
        fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
        let pid_file = bin.path.join("pid");
        let mut inner_env: Vec<(&str, PathBuf)> = Vec::new();
        if let Some((inner_name, inner_cwd)) = inner {
            let dir = bin.path.join("inner");
            fs::create_dir(&dir).unwrap();
            let inner_exe = dir.join(inner_name);
            fs::copy(std::env::current_exe().unwrap(), &inner_exe).unwrap();
            inner_env.push(("HELDOUT_CHILD_BIN", inner_exe));
            inner_env.push(("HELDOUT_CHILD_CWD", inner_cwd.to_path_buf()));
        }
        let run = "$HELDOUT_BIN agent_stand_in --exact --ignored --nocapture";
        // The trailing `:` keeps each shell alive as the parent instead of
        // exec'ing its last command. Behind shells, the tree is as deep as
        // a real one: terminal, login, shell, agent.
        let script = if behind_shells {
            format!("/bin/sh -c \"/bin/sh -c '{run}; :'; :\"; :")
        } else {
            format!("{run}; :")
        };
        let root = Command::new("/bin/sh")
            .arg("-c")
            .arg(script)
            .env("HELDOUT_BIN", &exe)
            .env("HELDOUT_PID_FILE", &pid_file)
            .envs(inner_env)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("sh starts");
        Tree {
            root,
            pid_file,
            _bin: bin,
        }
    }

    fn terminal_pid(&self) -> i32 {
        self.root.id() as i32
    }

    /// The stand-in's pid, once it is running.
    fn inner_pid(&self) -> i32 {
        wait_for("the stand-in process", START_WAIT, || {
            fs::read_to_string(&self.pid_file)
                .ok()
                .and_then(|s| s.trim().parse().ok())
        })
    }

    /// The pid of the stand-in that the stand-in runs, once it is running.
    fn nested_pid(&self) -> i32 {
        wait_for("the nested stand-in process", START_WAIT, || {
            fs::read_to_string(self.pid_file.with_extension("child"))
                .ok()
                .and_then(|s| s.trim().parse().ok())
        })
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        // Closing stdin ends the stand-in, and the shells after it.
        drop(self.root.stdin.take());
        let _ = self.root.wait();
    }
}

fn front(bundle_id: &str, name: &str, pid: i32) -> FrontApp {
    FrontApp {
        bundle_id: bundle_id.to_string(),
        name: name.to_string(),
        pid,
    }
}

fn ghostty(pid: i32) -> FrontApp {
    front("com.mitchellh.ghostty", "Ghostty", pid)
}

fn repo() -> Dir {
    repo_named("widget")
}

fn repo_named(name: &str) -> Dir {
    let mut dir = tempdir();
    dir.path = dir.path.join(name);
    write(&dir.path, "src/audio.rs", "");
    dir
}

fn session_under(tree: &Tree, front: &FrontApp) -> Session {
    tree.inner_pid();
    wait_for("detect to find the session", DETECT_WAIT, || detect(front))
}

#[test]
fn finds_claude_under_the_terminal_with_its_cwd() {
    let repo = repo();
    let tree = Tree::start("claude", &repo.path, false);
    let s = session_under(&tree, &ghostty(tree.terminal_pid()));
    assert_eq!(s.agent, Agent::Claude);
    assert_eq!(s.pid, tree.inner_pid());
    assert_eq!(s.cwd.canonicalize().unwrap(), repo.path);
    assert_eq!(s.project(), repo.path.file_name().unwrap().to_string_lossy());
}

#[test]
fn finds_codex_behind_the_login_and_the_shell() {
    let repo = repo();
    let tree = Tree::start("codex", &repo.path, true);
    let front = front("com.apple.Terminal", "Terminal", tree.terminal_pid());
    let s = session_under(&tree, &front);
    assert_eq!(s.agent, Agent::Codex);
    assert_eq!(s.pid, tree.inner_pid());
    assert_eq!(s.cwd.canonicalize().unwrap(), repo.path);
}

#[test]
fn a_front_app_that_is_not_a_terminal_has_no_session() {
    let repo = repo();
    let tree = Tree::start("claude", &repo.path, false);
    let pid = tree.terminal_pid();
    session_under(&tree, &ghostty(pid));
    assert_eq!(detect(&front("com.apple.Safari", "Safari", pid)), None);
}

#[test]
fn an_agent_under_another_terminal_is_not_this_ones() {
    let repo = repo();
    let with_agent = Tree::start("claude", &repo.path, false);
    session_under(&with_agent, &ghostty(with_agent.terminal_pid()));

    // A terminal running something that is not an agent.
    let other = Tree::start("notes", &repo.path, true);
    other.inner_pid();
    assert_eq!(detect(&ghostty(other.terminal_pid())), None);

    // And the first one is still found.
    let s = wait_for("detect to find the session", DETECT_WAIT, || detect(&ghostty(with_agent.terminal_pid())));
    assert_eq!(s.pid, with_agent.inner_pid());
}

#[test]
fn the_session_is_the_agent_not_what_it_runs_under_itself() {
    // claude in one repo, running another agent in a second directory (a
    // headless run it started, say). The session you are talking to is
    // the first.
    let repo = repo();
    let elsewhere = repo_named("elsewhere");
    for inner in ["claude", "codex"] {
        let tree = Tree::start_with("claude", &repo.path, true, Some((inner, &elsewhere.path)));
        let outer = tree.inner_pid();
        let nested = tree.nested_pid();
        assert_ne!(outer, nested);
        let s = wait_for("detect to find the session", DETECT_WAIT, || {
            detect(&ghostty(tree.terminal_pid()))
        });
        assert_eq!(s.pid, outer, "picked the agent's own child");
        assert_eq!(s.agent, Agent::Claude);
        assert_eq!(s.cwd.canonicalize().unwrap(), repo.path);
    }
}
