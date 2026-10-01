#!/usr/bin/env node
// Checks and prepares tasks (lib/workspace.mjs) without running an agent.
//
//   node task.mjs list
//   node task.mjs verify <id>            the checks at base and at the fix
//   node task.mjs warm <id>              runs the task's warm command at base and caches what it
//                                        built for every later run
//   node task.mjs prepare <id> [--ref base|fix|<commit>] [--hidden]
//                                        leaves a workspace and prints its path (the directory
//                                        above it is the run's {root})
//   node task.mjs regrade <set> [--only <text in "task config rN">]
//                                        runs the checks again on each run's saved diff
//
// verify builds the workspace twice. At `base` every check must come out as its expect_base says
// ("fail" for what the task asks for, "pass" for what must keep working), and at least one must
// fail. At `fix` every check must pass. It exits 0 only when both hold.
//
// regrade is for runs graded on a busy machine (many runs at once, and checks that wait on the
// real clock). One row at a time, it applies the run's diff to a fresh base workspace and runs the
// checks, then replaces the row's check_outputs (the first grading is kept as check_outputs_first)
// and prints what changed. Follow it with rescore.mjs, which turns check_outputs into fields,
// score and pass.

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { grade, listTasks, loadTask, prepare, resultsDir, ROOT, warm } from "./lib/workspace.mjs";

const [cmd, id, ...rest] = process.argv.slice(2);
const opt = (name, dflt) => {
  const i = rest.indexOf(`--${name}`);
  return i >= 0 ? rest[i + 1] : dflt;
};
// A run's {root}, and the workspace inside it.
const workspace = () => {
  const dir = path.join(fs.mkdtempSync(path.join(os.tmpdir(), "code-bench-")), "ws");
  fs.mkdirSync(dir);
  return dir;
};

if (cmd === "list") {
  for (const t of listTasks()) console.log(`${t.id}\t${t.set}\t${t.repo}${t.private ? " (private)" : ""}\t${t.kind ?? ""}\t${t.checks.length} checks\tbase ${t.base.slice(0, 7)} fix ${t.fix.slice(0, 7)}`);
} else if (cmd === "prepare") {
  const task = loadTask(id);
  const ref = { base: task.base, fix: task.fix }[opt("ref", "base")] ?? opt("ref");
  const dir = workspace();
  prepare(task, dir, ref);
  if (rest.includes("--hidden")) fs.cpSync(path.join(task.dir, "hidden"), dir, { recursive: true, force: true });
  console.log(dir);
} else if (cmd === "warm") {
  const task = loadTask(id);
  if (!task.warm) throw new Error(`${id}/task.json has no warm command`);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "code-bench-"));
  const kept = warm(task, root);
  fs.rmSync(root, { recursive: true, force: true });
  console.log(`cached ${kept.join(", ") || "nothing"} for ${id}`);
} else if (cmd === "regrade") {
  const only = opt("only");
  let done = 0;
  let changed = 0;
  for (const isPrivate of [false, true]) {
    const file = path.join(resultsDir(id, isPrivate), "runs.jsonl");
    if (!fs.existsSync(file)) continue;
    const rows = fs.readFileSync(file, "utf8").split("\n").filter(Boolean).map((l) => JSON.parse(l));
    for (const r of rows) {
      if (r.infra_error || !r.check_outputs || !r.patch_file || (only && !`${r.task} ${r.config} r${r.round}`.includes(only))) continue;
      const task = loadTask(r.task);
      const dir = workspace();
      const commit = prepare(task, dir);
      const patch = path.resolve(ROOT, r.patch_file);
      try {
        if (fs.statSync(patch).size) execFileSync("git", ["-C", dir, "apply", "--whitespace=nowarn", patch], { stdio: ["ignore", "pipe", "pipe"] });
      } catch (e) {
        console.log(`${r.task} ${r.config} r${r.round}: diff didn't apply, kept (${String(e.stderr ?? e.message).trim().slice(0, 200)})`);
        fs.rmSync(path.dirname(dir), { recursive: true, force: true });
        continue;
      }
      const g = grade(task, dir, commit);
      fs.rmSync(path.dirname(dir), { recursive: true, force: true });
      const flips = Object.keys(g.fields).filter((k) => g.fields[k] !== (r.check_outputs[k]?.exit === 0));
      done++;
      if (flips.length) changed++;
      console.log(`${r.task} ${r.config} r${r.round}: ${flips.length ? flips.map((k) => `${k} ${g.fields[k] ? "fail -> pass" : "pass -> fail"}`).join(", ") : "same"}`);
      r.check_outputs_first ??= r.check_outputs;
      r.check_outputs = g.outputs;
      r.regraded_at = new Date().toISOString();
      // Rewritten after every row, so a stopped regrade keeps what it has done.
      fs.writeFileSync(file, rows.map((x) => JSON.stringify(x)).join("\n") + "\n");
    }
  }
  console.log(`${done} rows regraded, ${changed} changed; run \`node rescore.mjs --set ${id}\` to apply`);
} else if (cmd === "verify") {
  const task = loadTask(id);
  let ok = true;
  for (const [label, ref] of [["base", task.base], ["fix", task.fix]]) {
    const dir = workspace();
    const commit = prepare(task, dir, ref);
    const g = grade(task, dir, commit);
    console.log(`\n${label} (${ref.slice(0, 7)})`);
    for (const c of task.checks) {
      const want = label === "fix" ? true : (c.expect_base ?? "fail") === "pass";
      const got = g.fields[c.name];
      const good = got === want;
      if (!good) ok = false;
      console.log(`  ${good ? "ok " : "BAD"} ${c.name}: ${got ? "pass" : "fail"} (expected ${want ? "pass" : "fail"}, ${g.outputs[c.name].ms} ms)`);
      if (!good) console.log(g.outputs[c.name].tail.replace(/^/gm, "      "));
    }
    for (const e of g.errors) {
      ok = false;
      console.log(`  BAD ${e}`);
    }
    if (label === "base" && Object.values(g.fields).every(Boolean)) {
      ok = false;
      console.log("  BAD every check passes at base, so the task asks for nothing");
    }
    fs.rmSync(path.dirname(dir), { recursive: true, force: true });
  }
  console.log(ok ? "\nverified" : "\nNOT verified");
  process.exit(ok ? 0 : 1);
} else {
  console.error("usage: task.mjs list | verify <id> | warm <id> | prepare <id> [--ref base|fix|<commit>] [--hidden] | regrade <set> [--only <text>]");
  process.exit(2);
}
