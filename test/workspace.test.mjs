// The workspace and grading (lib/workspace.mjs): the export has one commit and no history, the
// diff covers committed and uncommitted changes, protected files go back to base, hidden files are
// copied in before the checks, warm caches what a task builds, private tasks are found beside the
// public ones, and a run that reads a source checkout is caught.
//   node --test test/*.test.mjs

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { test } from "node:test";

const tmp = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), "code-bench-test-")));
const src = path.join(tmp, "src");
const root = path.join(tmp, "bench");
const git = (dir, ...args) => execFileSync("git", ["-C", dir, "-c", "user.name=t", "-c", "user.email=t@localhost", ...args], { encoding: "utf8" }).trim();
const write = (file, text) => (fs.mkdirSync(path.dirname(file), { recursive: true }), fs.writeFileSync(file, text));

// A source repo with a base commit and a fix, and one task over it.
fs.mkdirSync(src);
git(src, "init", "-q", "-b", "main");
write(path.join(src, "add.mjs"), "export const add = (a, b) => a - b;\n");
write(path.join(src, "old.test.mjs"), "import './add.mjs';\n");
git(src, "add", "-A");
git(src, "commit", "-q", "-m", "base");
const base = git(src, "rev-parse", "HEAD");
write(path.join(src, "add.mjs"), "export const add = (a, b) => a + b;\n");
git(src, "commit", "-q", "-am", "the fix");
const fix = git(src, "rev-parse", "HEAD");
write(path.join(src, "untracked/dep.txt"), "dep\n");

const toy = {
  id: "toy-add", set: "easy", repo: "toy", base, fix, prompt: "add() subtracts", clone: ["untracked"], protected: ["old.test.mjs"],
  env: { BUILD_DIR: "{root}/build" }, warm: 'mkdir -p "$BUILD_DIR" && echo built > "$BUILD_DIR/dep.o"',
  checks: [
    { name: "adds", cmd: "node hidden.test.mjs", expect_base: "fail" },
    { name: "old_test", cmd: "node old.test.mjs", expect_base: "pass" },
    { name: "build_dir_set", cmd: 'test "${BUILD_DIR##*/}" = build', expect_base: "pass" },
  ],
};
write(path.join(root, "repos.json"), JSON.stringify({ toy: { path: src } }));
write(path.join(root, "tasks/toy-add/task.json"), JSON.stringify(toy));
write(path.join(root, "tasks/toy-add/hidden/hidden.test.mjs"), "import { add } from './add.mjs';\nif (add(2, 3) !== 5) process.exit(1);\n");
write(path.join(root, "private/tasks/toy-private/task.json"), JSON.stringify({ ...toy, id: "toy-private", set: "hard" }));

process.env.CODE_BENCH_DIR = root;
const { grade, leaks, listTasks, loadTask, prepare, resultsDir, runEnv, score, scoredFields, warm } = await import("../lib/workspace.mjs");
const workspace = () => {
  const dir = path.join(fs.mkdtempSync(path.join(tmp, "run-")), "ws");
  fs.mkdirSync(dir);
  return dir;
};

test("tasks are found in tasks/ and private/tasks/, and a private task's results go under private/", () => {
  assert.deepEqual(listTasks().map((t) => [t.id, t.set, t.private]), [["toy-add", "easy", false], ["toy-private", "hard", true]]);
  assert.equal(resultsDir("hard", loadTask("toy-private").private), path.join(root, "private/results/hard"));
  assert.equal(resultsDir("easy", loadTask("toy-add").private), path.join(root, "results/easy"));
  assert.throws(() => loadTask("nope"), /no task nope/);
});

test("the workspace is the base commit with one commit of history and the cloned paths", () => {
  const task = loadTask("toy-add");
  const dir = workspace();
  const commit = prepare(task, dir);
  assert.match(fs.readFileSync(path.join(dir, "add.mjs"), "utf8"), /a - b/);
  assert.equal(fs.readFileSync(path.join(dir, "untracked/dep.txt"), "utf8"), "dep\n");
  assert.equal(git(dir, "rev-list", "--count", "HEAD"), "1");
  assert.equal(git(dir, "rev-parse", "HEAD"), commit);
  assert.doesNotMatch(git(dir, "log", "--all", "--format=%s"), /the fix/);
});

test("an untouched base fails the task's check and keeps the old test passing; the fix passes both", () => {
  const task = loadTask("toy-add");
  const dir = workspace();
  const atBase = grade(task, dir, prepare(task, dir));
  assert.deepEqual(atBase.fields, { adds: false, old_test: true, build_dir_set: true });
  assert.equal(atBase.patch, "");
  assert.deepEqual(score(atBase.fields), { score: 0.667, pass: false });
  const fixed = workspace();
  const atFix = grade(task, fixed, prepare(task, fixed, task.fix));
  assert.deepEqual(atFix.fields, { adds: true, old_test: true, build_dir_set: true });
  assert.deepEqual(score(atFix.fields), { score: 1, pass: true });
});

test("the diff counts committed and uncommitted work, and a weakened protected test is put back", () => {
  const task = loadTask("toy-add");
  const dir = workspace();
  const commit = prepare(task, dir);
  write(path.join(dir, "add.mjs"), "export const add = (a, b) => a + b;\n");
  git(dir, "commit", "-q", "-am", "agent commit");
  write(path.join(dir, "notes.txt"), "one\ntwo\n");
  write(path.join(dir, "old.test.mjs"), "process.exit(1);\n");
  const g = grade(task, dir, commit);
  assert.deepEqual(g.fields, { adds: true, old_test: true, build_dir_set: true });
  assert.deepEqual(g.files.map((f) => f.file).sort(), ["add.mjs", "notes.txt", "old.test.mjs"]);
  assert.match(g.patch, /\+export const add = \(a, b\) => a \+ b;/);
  assert.ok(!g.files.some((f) => f.file === "hidden.test.mjs"), "hidden files aren't part of the run's diff");
});

test("warm caches what the task's command built beside the workspace, and later runs start with a copy", () => {
  const task = loadTask("toy-add");
  assert.deepEqual(warm(task, fs.mkdtempSync(path.join(tmp, "warm-"))), ["build"]);
  const dir = workspace();
  prepare(task, dir);
  assert.equal(fs.readFileSync(path.join(path.dirname(dir), "build/dep.o"), "utf8"), "built\n");
  assert.deepEqual(runEnv(task, "/r"), { BUILD_DIR: "/r/build" });
  assert.ok(!fs.existsSync(path.join(dir, "build")), "the cache is outside the workspace, so not in the diff");
});

test("a task whose task.json doesn't parse is skipped, not fatal", () => {
  write(path.join(root, "tasks/half-written/task.json"), '{ "id": "half-wr');
  assert.deepEqual(listTasks().map((t) => t.id), ["toy-add", "toy-private"]);
  fs.rmSync(path.join(root, "tasks/half-written"), { recursive: true });
});

test("reading a source checkout costs the run its score; a remote pattern in a command only marks it", () => {
  const call = (command) => ({ name: "Bash", input: { command } });
  const fields = { adds: true, old_test: true };
  assert.equal(leaks([call("node --test"), call("git log --oneline"), { name: "Read", input: { file_path: "/tmp/ws/add.mjs" } }]).length, 0);
  assert.equal(leaks([call(`ls ${src}-other/eval`)]).length, 0, "a sibling directory that starts with the checkout's name");
  assert.deepEqual(leaks([call(`cat ${src}/add.mjs`)]).map((l) => l.kind), ["checkout"]);
  assert.deepEqual(leaks([{ name: "Read", input: { file_path: src } }]).map((l) => l.kind), ["checkout"]);
  assert.deepEqual(scoredFields(fields, [call(`git -C ${src} log`)]), { adds: false, old_test: false, no_lookup: false });
  assert.deepEqual(leaks([call("git clone https://github.com/x/y")]).map((l) => l.kind), ["remote"]);
  assert.deepEqual(leaks([call("gh api repos/x/y/commits")]).map((l) => l.kind), ["remote"]);
  assert.deepEqual(scoredFields(fields, [call("echo https://github.com/x/y > README.md")]), fields);
  assert.equal(leaks([{ name: "Write", input: { content: "see https://github.com/x/y" } }]).length, 0, "remote patterns are for commands");
});

test.after(() => fs.rmSync(tmp, { recursive: true, force: true }));
