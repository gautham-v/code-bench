#!/usr/bin/env node
// Runs every (task, config) of a set with `claude -p --model <id> --effort <level>` and appends one
// JSON line per run to results/<set>/runs.jsonl (private/results/<set>/ for a private task).
//
//   node bench.mjs --set hard --configs sonnet:high,opus:high [--rounds 2] [--concurrency 4]
//                  [--tasks id,id] [--run-timeout-min 75] [--max-minutes 110] [--seed 7] [--dry-run]
//
// A config is model:effort. The model is an alias below or a full model id, so a new model needs
// no change here: --configs claude-some-new-model:high.
//
// A run gets an export of the task's base commit as its cwd, the tools Bash, Read, Edit, Write,
// Glob and Grep, and no network tools. After it exits, the worker copies in the task's held-out
// tests and runs its checks (lib/workspace.mjs). Each run's diff is kept in patches/ and its raw
// stream-json in streams/.
//
// Schedule: round-robin. Round r runs every (task, config) once, in an order shuffled with a seed
// derived from --seed and r, and round r+1 starts only after every cell of round r has started.
// Cells that already have a row are skipped, so adding a config or a round to a set runs only what
// is missing.
//
// Resumable: the scheduler keeps up to --concurrency runs going until everything is done or
// --max-minutes is up. Each run is its own detached worker process (this file with --one), so a
// run still going when the scheduler exits records itself, and the next invocation counts it.

import { spawn, execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { answerOf, childEnv, summarize, traceOf } from "./lib/stream.mjs";
import { grade, leaks, listTasks, loadTask, prepare, resultsDir, ROOT, runEnv, score, scoredFields, SETS } from "./lib/workspace.mjs";

const SELF = fileURLToPath(import.meta.url);
const argv = process.argv.slice(2);
const opt = (name, dflt) => {
  const i = argv.indexOf(`--${name}`);
  return i >= 0 ? argv[i + 1] : dflt;
};
const flag = (name) => argv.includes(`--${name}`);

export const MODELS = {
  haiku: "claude-haiku-4-5-20251001",
  sonnet: "claude-sonnet-5-5",
  opus: "claude-opus-5-5",
  fable: "claude-fable-5-1",
};
const parseConfig = (c) => {
  const [m, effort = "default"] = c.split(":");
  const model = MODELS[m] ?? m;
  return { id: `${model}/${effort}`, model, effort };
};

// Every prompt ends with these.
const RULES = `

Work in the current directory, which is a git checkout of the project. Don't use the network, and don't look for other copies of this project on this machine or online. Your change will be checked by tests you can't see, and the tests that already pass must keep passing. Those tests call the project's existing functions and types as they are named today, so don't rename or re-sign what exists unless the task needs it.

When you are finished, end your last message with one JSON object: {"status": "done" | "partial" | "blocked", "summary": "<one sentence>"}`;

const TOOLS = "Bash,Read,Edit,Write,Glob,Grep";
const RUN_CAP_MIN = Number(opt("run-timeout-min", 75));
const CLAUDE = process.env.CLAUDE_BIN || "claude";

const cellKey = (c) => `${c.task}|${c.config}|${c.round}`;
const fileKey = (c) => cellKey(c).replace(/[^\w.-]+/g, "_");
const runsFile = (set, isPrivate) => path.join(resultsDir(set, isPrivate), "runs.jsonl");

export function readRuns(set) {
  return [false, true].flatMap((p) => {
    const f = runsFile(set, p);
    if (!fs.existsSync(f)) return [];
    return fs.readFileSync(f, "utf8").split("\n").filter(Boolean).flatMap((l) => {
      try {
        return [JSON.parse(l)];
      } catch {
        return [];
      }
    });
  });
}

// How many other runs of this benchmark were going, sampled from `ps`.
function siblings(selfPid) {
  try {
    const out = execFileSync("ps", ["-Ao", "pid=,command="], { encoding: "utf8", maxBuffer: 16 << 20 });
    return out.split("\n").filter((l) => /bench\.mjs --one/.test(l) && Number(l.trim().split(/\s+/)[0]) !== selfPid).length;
  } catch {
    return null;
  }
}

// ---- worker: one run -----------------------------------------------------------------------

async function runOne(cell) {
  const task = loadTask(cell.task);
  const config = parseConfig(cell.config.replace("/", ":"));
  const dir = resultsDir(task.set, task.private);
  for (const d of ["streams", "patches"]) fs.mkdirSync(path.join(dir, d), { recursive: true });
  const inflightFile = path.join(resultsDir(task.set), "inflight", `${fileKey(cell)}.json`);
  fs.mkdirSync(path.dirname(inflightFile), { recursive: true });

  // The run's {root}: the workspace, the (empty) MCP config and whatever the task's env puts there.
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "code-bench-"));
  const mcpConfig = path.join(root, "mcp.json");
  fs.writeFileSync(mcpConfig, JSON.stringify({ mcpServers: {} }));
  const cwd = path.join(root, "ws");
  fs.mkdirSync(cwd);
  const baseCommit = prepare(task, cwd);

  const started = Date.now();
  fs.writeFileSync(inflightFile, JSON.stringify({ ...cell, pid: process.pid, started_at: new Date(started).toISOString() }));
  const streamFile = path.join(dir, "streams", `${fileKey(cell)}.jsonl`);
  const streamOut = fs.createWriteStream(streamFile);
  const args = [
    "-p", task.prompt + RULES,
    "--output-format", "stream-json", "--verbose",
    "--model", config.model,
    "--strict-mcp-config", "--mcp-config", mcpConfig,
    "--tools", TOOLS, "--allowedTools", TOOLS, "--disallowedTools", "WebFetch,WebSearch",
    "--no-session-persistence",
  ];
  if (config.effort !== "default") args.push("--effort", config.effort);
  const proc = spawn(CLAUDE, args, { cwd, env: childEnv(runEnv(task, root)), stdio: ["ignore", "pipe", "pipe"] });
  const events = [];
  let buf = "";
  let stderr = "";
  proc.stdout.setEncoding("utf8");
  proc.stdout.on("data", (chunk) => {
    streamOut.write(chunk);
    buf += chunk;
    let nl;
    while ((nl = buf.indexOf("\n")) >= 0) {
      const line = buf.slice(0, nl);
      buf = buf.slice(nl + 1);
      if (!line.trim()) continue;
      try {
        const e = JSON.parse(line);
        e._t = Date.now() - started;
        events.push(e);
      } catch {}
    }
  });
  proc.stderr.on("data", (d) => (stderr += d));

  const load = [];
  const sample = () => {
    const n = siblings(process.pid);
    if (n != null) load.push(n);
  };
  sample();
  const sampler = setInterval(sample, 15_000);

  let timedOut = false;
  const timer = setTimeout(() => {
    timedOut = true;
    proc.kill("SIGTERM");
    setTimeout(() => proc.kill("SIGKILL"), 5000).unref();
  }, RUN_CAP_MIN * 60_000);
  const exit = await new Promise((resolve) => proc.on("exit", (code, signal) => resolve({ code, signal })));
  clearTimeout(timer);
  clearInterval(sampler);
  streamOut.end();
  const wall_ms = Date.now() - started;

  const m = summarize(events);
  const graded = grade(task, cwd, baseCommit);
  const answer = answerOf(m.final_text, events);
  const trace = traceOf(events);
  const fields = scoredFields(graded.fields, trace);
  let { score: sc, pass } = score(fields);
  if (timedOut) [sc, pass] = [0, false];
  const rateLimited = events.some((e) => e.type === "rate_limit_event" && e.rate_limit_info?.status && e.rate_limit_info.status !== "allowed");
  // No result event and not a timeout: the CLI or API failed (auth, rate limit, crash), not the
  // model. Such rows are kept but don't count as done, so the cell runs again.
  const infraError = !timedOut && m.result_subtype == null;
  const errors = [...graded.errors];
  if (timedOut) errors.push("run_timeout");
  if (exit.code !== 0 && !timedOut) errors.push(`exit ${exit.code ?? exit.signal}`);
  if (m.is_error) errors.push(`result ${m.result_subtype}`);
  if (!answer) errors.push("no JSON answer");
  if (stderr.trim()) errors.push(`stderr: ${stderr.trim().slice(0, 300)}`);

  const patchFile = path.join(dir, "patches", `${fileKey(cell)}.diff`);
  fs.writeFileSync(patchFile, graded.patch);
  const row = {
    task: task.id, set: task.set, config: config.id, model: config.model, effort: config.effort, round: cell.round, order: cell.order,
    started_at: new Date(started).toISOString(), wall_ms, first_tool_ms: m.first_tool_ms,
    timeout: timedOut, infra_error: infraError, rate_limited: rateLimited,
    score: sc, pass, fields,
    // What the run said about itself, and whether it said done when a check failed.
    claimed: answer?.status ?? null, false_success: answer?.status === "done" && !pass, summary: answer?.summary ?? null,
    lookups: leaks(trace),
    files_changed: graded.files.length,
    lines_added: graded.files.reduce((a, f) => a + f.added, 0),
    lines_removed: graded.files.reduce((a, f) => a + f.removed, 0),
    files: graded.files, check_outputs: graded.outputs,
    patch_file: path.relative(ROOT, patchFile), stream_file: path.relative(ROOT, streamFile),
    model_reported: m.model, per_turn_effort_active: m.per_turn_effort_active,
    turns: m.turns, assistant_messages: m.assistant_messages, thinking_blocks: m.thinking_blocks,
    tool_calls: m.tool_calls, tool_calls_by_tool: m.tool_calls_by_tool, tool_errors: m.tool_errors,
    repeated_calls: m.repeated_calls, retries_after_error: m.retries_after_error, tool_result_chars: m.tool_result_chars,
    usage: m.usage, model_usage: m.model_usage, cost_usd: m.cost_usd, duration_ms: m.duration_ms, duration_api_ms: m.duration_api_ms,
    load_siblings: load.length ? +(load.reduce((a, b) => a + b, 0) / load.length).toFixed(2) : null,
    errors,
  };
  fs.appendFileSync(runsFile(task.set, task.private), JSON.stringify(row) + "\n");
  fs.rmSync(inflightFile, { force: true });
  fs.rmSync(root, { recursive: true, force: true });
  console.log(`done ${cellKey(cell)} score=${sc} pass=${pass} ${(wall_ms / 1000).toFixed(0)}s tools=${m.tool_calls} cost=${m.cost_usd?.toFixed(3)}${errors.length ? " errors=" + errors.join("; ").slice(0, 200) : ""}`);
}

// ---- scheduler -----------------------------------------------------------------------------

// Seeded PRNG (mulberry32) and Fisher-Yates shuffle.
function rng(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
function shuffle(xs, seed) {
  const r = rng(seed);
  const a = [...xs];
  for (let i = a.length - 1; i > 0; i--) {
    const j = Math.floor(r() * (i + 1));
    [a[i], a[j]] = [a[j], a[i]];
  }
  return a;
}

const alive = (pid) => {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
};

// A cell is done when it has a row that isn't an infrastructure failure, or after 3 failures.
function doneKeys(rows) {
  const ok = new Set();
  const fails = {};
  for (const r of rows) {
    const k = cellKey(r);
    if (!r.infra_error) ok.add(k);
    else fails[k] = (fails[k] ?? 0) + 1;
  }
  for (const [k, n] of Object.entries(fails)) if (n >= 3) ok.add(k);
  return ok;
}

async function schedule() {
  const SET = opt("set");
  if (!SETS.includes(SET)) throw new Error(`--set must be one of ${SETS.join(", ")}`);
  if (!opt("configs")) throw new Error("--configs model:effort[,model:effort...] is required");
  const MAX_MIN = Number(opt("max-minutes", 110));
  const CONCURRENCY = Number(opt("concurrency", 4));
  const ROUNDS = Number(opt("rounds", 2));
  const SEED = Number(opt("seed", 7));
  const onlyTasks = opt("tasks")?.split(",");
  const configs = opt("configs").split(",").map(parseConfig);
  const tasks = listTasks().filter((t) => t.set === SET && (!onlyTasks || onlyTasks.includes(t.id)));
  const dir = resultsDir(SET);
  const INFLIGHT = path.join(dir, "inflight");
  const LOGS = path.join(dir, "logs");
  const t0 = Date.now();
  const deadline = t0 + MAX_MIN * 60_000;
  const log = (...a) => console.log(`[${((Date.now() - t0) / 1000).toFixed(0).padStart(4)}s]`, ...a);

  const cells = [];
  for (let round = 1; round <= ROUNDS; round++) {
    const base = tasks.flatMap((t) => configs.map((c) => ({ task: t.id, config: c.id })));
    shuffle(base, SEED * 1000 + round).forEach((c, i) => cells.push({ ...c, round, order: i }));
  }
  const inflight = () => {
    if (!fs.existsSync(INFLIGHT)) return [];
    const out = [];
    for (const f of fs.readdirSync(INFLIGHT)) {
      const p = path.join(INFLIGHT, f);
      try {
        const x = JSON.parse(fs.readFileSync(p, "utf8"));
        if (alive(x.pid)) out.push(x);
        else fs.rmSync(p, { force: true }); // the worker died without recording; the cell runs again
      } catch {}
    }
    return out;
  };
  const pendingNow = () => {
    const done = doneKeys(readRuns(SET));
    const running = new Set(inflight().map(cellKey));
    return cells.filter((c) => !done.has(cellKey(c)) && !running.has(cellKey(c)));
  };
  let pending = pendingNow();
  log(`${SET}: ${cells.length} cells (${tasks.length} tasks x ${configs.length} configs x ${ROUNDS} rounds), ${cells.length - pending.length - inflight().length} done, ${inflight().length} running, ${pending.length} pending; concurrency ${CONCURRENCY}`);
  log(`configs: ${configs.map((c) => c.id).join(", ")}`);
  if (flag("dry-run")) {
    for (const c of pending.slice(0, 40)) log("pending", cellKey(c));
    if (pending.length > 40) log(`... and ${pending.length - 40} more`);
    return;
  }

  const lock = path.join(dir, "scheduler.lock");
  fs.mkdirSync(LOGS, { recursive: true });
  fs.mkdirSync(INFLIGHT, { recursive: true });
  if (fs.existsSync(lock) && alive(Number(fs.readFileSync(lock, "utf8")))) {
    log("another scheduler is running for this set; exiting");
    return;
  }
  fs.writeFileSync(lock, String(process.pid));
  let started = 0;
  try {
    for (;;) {
      pending = pendingNow();
      const running = inflight();
      if (!pending.length && !running.length) break;
      if (deadline - Date.now() < 20_000) break;
      // Keep rounds whole: start a round-r+1 cell only once every round-r cell has started.
      const next = pending.find((c) => c.round === Math.min(...pending.map((p) => p.round)));
      if (next && running.length < CONCURRENCY) {
        const logFd = fs.openSync(path.join(LOGS, `${fileKey(next)}.log`), "a");
        const w = spawn(process.execPath, [SELF, "--one", JSON.stringify(next), "--run-timeout-min", String(RUN_CAP_MIN)], { detached: true, stdio: ["ignore", logFd, logFd], env: childEnv() });
        w.unref();
        fs.closeSync(logFd);
        // Mark it right away so the next loop doesn't start it twice before the worker writes.
        fs.writeFileSync(path.join(INFLIGHT, `${fileKey(next)}.json`), JSON.stringify({ ...next, pid: w.pid, started_at: new Date().toISOString() }));
        started++;
        log(`start ${cellKey(next)} (pid ${w.pid})`);
        await new Promise((r) => setTimeout(r, 3000)); // stagger starts
        continue;
      }
      await new Promise((r) => setTimeout(r, 5000));
    }
  } finally {
    fs.rmSync(lock, { force: true });
  }
  const done = doneKeys(readRuns(SET));
  const left = cells.filter((c) => !done.has(cellKey(c))).length;
  log(`started ${started}; ${cells.length - left}/${cells.length} done, ${inflight().length} still running (they record themselves), ${left} not done${left ? " (run again to continue)" : " (complete)"}`);
}

if (process.argv[1] === SELF) {
  if (opt("one")) {
    await runOne(JSON.parse(opt("one")));
    process.exit(0);
  } else {
    await schedule();
  }
}
