// A task's workspace and its grading.
//
// A task is a directory tasks/<id>/ (or private/tasks/<id>/, for tasks from repos that aren't
// public) with task.json and hidden/:
//   id         the directory's name
//   set        which benchmark the task belongs to. There is one, "hard"; any other value keeps a
//              task out of the runs
//   repo       a key of repos.json (name -> path of a local git checkout)
//   base       the commit the agent starts from
//   fix        the reference solution's commit (used by `task.mjs verify`, never shown)
//   kind       bug | feature | refactor | spec
//   prompt     what the agent is asked, written as a user would ask it
//   clone      untracked paths copied from the checkout into the workspace (["node_modules"])
//   env        environment for the agent and the checks. {root} is the run's own temp directory,
//              whose ws/ is the workspace: { "CARGO_TARGET_DIR": "{root}/target" } gives each run
//              its own build directory outside the diff
//   warm       a command that fills what env points at (cargo test --no-run). `task.mjs warm` runs
//              it once at base and keeps what it left in {root} under .cache/<id>/, and every
//              run's {root} starts with a copy, so runs don't each compile the dependencies
//   protected  tracked files put back to their base content before grading, so a run can't pass
//              by weakening a test that already existed
//   checks     [{ name, cmd, expect_base, timeout_s, gate }]: shell commands run in the workspace
//              after hidden/ is copied over it. Exit 0 meets the sub-goal. expect_base is "fail"
//              for what the task asks for and "pass" for what must keep working. timeout_s
//              defaults to 600. A gate is a check every model measured so far has met: a run must
//              still meet it to pass, but it earns no score, so the score is over what models
//              have failed.
//   hidden/    the held-out files (tests), at their paths relative to the repo root
//
// The workspace is an export of `base` with a fresh one-commit history, so the fix isn't in its
// git objects.

import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

// CODE_BENCH_DIR points everything at another root (the tests use one).
export const ROOT = process.env.CODE_BENCH_DIR ? path.resolve(process.env.CODE_BENCH_DIR) : path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const TASK_DIRS = [
  { dir: path.join(ROOT, "tasks"), private: false },
  { dir: path.join(ROOT, "private/tasks"), private: true },
];
export const SETS = ["hard"];

// Where a set's runs are recorded. Rows, diffs and transcripts of a private task hold its repo's
// code, so they go under private/, which this repo doesn't track.
export const resultsDir = (set, isPrivate = false) => path.join(ROOT, isPrivate ? "private/results" : "results", set);

const sh = (cmd, args, opts = {}) => execFileSync(cmd, args, { encoding: "utf8", maxBuffer: 256 << 20, stdio: ["ignore", "pipe", "pipe"], ...opts });
const git = (dir, ...args) => sh("git", ["-C", dir, ...args]);

const repos = () => JSON.parse(fs.readFileSync(path.join(ROOT, "repos.json"), "utf8"));
export function repoPath(name) {
  const r = repos()[name];
  if (!r) throw new Error(`repos.json has no "${name}"`);
  return path.resolve(ROOT, r.path.replace(/^~(?=\/)/, process.env.HOME));
}

export function loadTask(id) {
  const home = TASK_DIRS.find((t) => fs.existsSync(path.join(t.dir, id, "task.json")));
  if (!home) throw new Error(`no task ${id} in tasks/ or private/tasks/`);
  const dir = path.join(home.dir, id);
  const task = JSON.parse(fs.readFileSync(path.join(dir, "task.json"), "utf8"));
  if (task.id !== id) throw new Error(`${id}/task.json has id ${task.id}`);
  for (const k of ["set", "repo", "base", "fix", "prompt", "checks"]) if (!task[k]) throw new Error(`${id}/task.json: missing ${k}`);
  if (!task.checks.length) throw new Error(`${id}/task.json: no checks`);
  return { clone: [], protected: [], ...task, dir, private: home.private };
}

// A task that doesn't load (one being written) is skipped, so it can't stop the others' runs.
export const listTasks = () =>
  TASK_DIRS.flatMap((t) => (fs.existsSync(t.dir) ? fs.readdirSync(t.dir).filter((d) => !d.startsWith(".") && fs.existsSync(path.join(t.dir, d, "task.json"))) : []))
    .sort()
    .flatMap((d) => {
      try {
        return [loadTask(d)];
      } catch (e) {
        console.error(`skipping ${d}: ${e.message}`);
        return [];
      }
    });

export const cacheDir = (task) => path.join(ROOT, ".cache", task.id);

// The task's env with {root} filled in; root is the directory that holds the workspace.
export const runEnv = (task, root) => Object.fromEntries(Object.entries(task.env ?? {}).map(([k, v]) => [k, String(v).replaceAll("{root}", root)]));

// Partial credit: the share of the task's scored checks met (its gates earn nothing). A run passes
// when it meets every check, gates included.
export function score(fields, task) {
  const gates = new Set((task?.checks ?? []).filter((c) => c.gate).map((c) => c.name));
  const all = Object.values(fields);
  const scored = Object.entries(fields).filter(([k]) => !gates.has(k)).map(([, v]) => v);
  const s = scored.length ? scored.filter(Boolean).length / scored.length : 0;
  return { score: Math.round(s * 1000) / 1000, pass: all.length > 0 && all.every(Boolean), gates_failed: [...gates].filter((k) => fields[k] === false).length };
}

// -c clones the files on APFS (copy-on-write), so a run can't change the original and a large
// directory costs no disk; -p keeps the times cargo's fingerprints compare.
function copyTree(from, to) {
  try {
    sh("cp", ["-cRp", from, to]);
  } catch {
    sh("cp", ["-Rp", from, to]);
  }
}

// Exports `ref` of the task's repo into `dir` (which must exist and be empty) and commits it as
// the only commit. Returns that commit, which grade() diffs against. The directory above `dir` is
// the run's {root}: what `warm` cached for the task is copied there.
export function prepare(task, dir, ref = task.base, { cache = true } = {}) {
  const src = repoPath(task.repo);
  const tar = sh("git", ["-C", src, "archive", ref], { encoding: "buffer" });
  // -m: the files are as new as the export, not as old as the commit. A warmed build is newer
  // than the commit, and cargo would take the base's cached binary for any other ref's source.
  sh("tar", ["-x", "-m", "-C", dir], { input: tar, stdio: ["pipe", "pipe", "pipe"] });
  for (const p of task.clone) {
    const from = path.join(src, p);
    if (!fs.existsSync(from)) throw new Error(`${task.id}: ${from} doesn't exist (clone)`);
    copyTree(from, path.join(dir, p));
  }
  if (cache && fs.existsSync(cacheDir(task))) for (const e of fs.readdirSync(cacheDir(task))) copyTree(path.join(cacheDir(task), e), path.join(path.dirname(dir), e));
  git(dir, "init", "-q", "-b", "main");
  git(dir, "add", "-A");
  git(dir, "-c", "user.name=eval", "-c", "user.email=eval@localhost", "commit", "-q", "-m", "Import");
  return git(dir, "rev-parse", "HEAD").trim();
}

// What the run changed since `baseCommit` (committed or not), then the checks: protected files
// are put back, hidden/ is copied over the workspace, and each check's command runs.
export function grade(task, dir, baseCommit) {
  const errors = [];
  let patch = "";
  let files = [];
  try {
    git(dir, "add", "-A");
    patch = git(dir, "diff", "--cached", baseCommit);
    files = git(dir, "diff", "--cached", "--numstat", baseCommit)
      .split("\n")
      .filter(Boolean)
      .map((l) => {
        const [added, removed, file] = l.split("\t");
        return { file, added: Number(added) || 0, removed: Number(removed) || 0 };
      });
  } catch (e) {
    errors.push(`diff: ${String(e.message ?? e).slice(0, 200)}`);
  }
  for (const p of task.protected) {
    try {
      git(dir, "checkout", baseCommit, "--", p);
    } catch (e) {
      errors.push(`protected ${p}: ${String(e.message ?? e).slice(0, 200)}`);
    }
  }
  const hidden = path.join(task.dir, "hidden");
  if (fs.existsSync(hidden)) fs.cpSync(hidden, dir, { recursive: true, force: true });
  const fields = {};
  const outputs = {};
  for (const c of task.checks) {
    const t0 = Date.now();
    const r = spawnSync("bash", ["-c", c.cmd], { cwd: dir, encoding: "utf8", timeout: (c.timeout_s ?? 600) * 1000, maxBuffer: 64 << 20, env: { ...process.env, CI: "1", ...runEnv(task, path.dirname(dir)) } });
    fields[c.name] = r.status === 0;
    outputs[c.name] = { exit: r.status, signal: r.signal ?? null, ms: Date.now() - t0, tail: `${r.stdout ?? ""}\n${r.stderr ?? ""}`.trim().slice(-1500) };
  }
  return { fields, outputs, patch, files, errors };
}

// A run may not look the fix up: the checkouts the tasks come from, and their remotes. Returns the
// calls that look like it. kind "checkout" (the call names a checkout's path) is certain and costs
// the run its score; kind "remote" is a pattern in a command and only marks the row for a look,
// since a test fixture's README can hold a github.com link.
export function leaks(trace) {
  // A checkout's path, but not a sibling directory whose name starts with it.
  const paths = Object.keys(repos()).map((n) => new RegExp(repoPath(n).replace(/[.*+?^${}()|[\]\\]/g, "\\$&") + "(?![\\w.-])"));
  // The remote patterns apply to commands only: an edit may well contain a github.com link.
  const remote = [/github\.com/i, /\bgh\s+(api|repo|pr|issue|search|browse)\b/, /\bgit\s+(clone|fetch|pull|remote\s+add)\b/];
  const out = [];
  for (const c of trace) {
    const text = JSON.stringify(c.input ?? {});
    const local = paths.find((re) => re.test(text));
    const hit = local ?? (c.name === "Bash" ? remote.find((re) => re.test(text)) : null);
    if (hit) out.push({ kind: local ? "checkout" : "remote", name: c.name, matched: local ? "a source checkout" : String(hit) });
  }
  return out;
}

// The fields a run is scored on: its checks, all failed if it read a source checkout.
export function scoredFields(fields, trace = []) {
  if (!leaks(trace).some((l) => l.kind === "checkout")) return fields;
  return { ...Object.fromEntries(Object.keys(fields).map((k) => [k, false])), no_lookup: false };
}

// Runs the task's warm command in a base workspace and keeps what it left beside the workspace
// (the build directory env points at) as the task's cache.
export function warm(task, root) {
  const dir = path.join(root, "ws");
  fs.mkdirSync(dir);
  prepare(task, dir, task.base, { cache: false });
  const r = spawnSync("bash", ["-c", task.warm], { cwd: dir, encoding: "utf8", maxBuffer: 64 << 20, env: { ...process.env, CI: "1", ...runEnv(task, root) } });
  if (r.status !== 0) throw new Error(`warm failed (exit ${r.status}):\n${`${r.stdout}\n${r.stderr}`.trim().slice(-2000)}`);
  fs.rmSync(cacheDir(task), { recursive: true, force: true });
  fs.mkdirSync(cacheDir(task), { recursive: true });
  const kept = fs.readdirSync(root).filter((e) => e !== "ws");
  for (const e of kept) copyTree(path.join(root, e), path.join(cacheDir(task), e));
  return kept;
}
