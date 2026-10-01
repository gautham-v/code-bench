#!/usr/bin/env node
// Regrades recorded runs from each check's recorded exit code and the run's tool calls (rebuilt
// from its raw stream), and rewrites fields, score, pass, false_success and lookups in place. The
// checks themselves are not run again (task.mjs regrade does that). Use it after regrade, or after
// a change to the lookup guard. Timeouts stay at 0.
//
//   node rescore.mjs [--set hard] [--dry-run]

import fs from "node:fs";
import path from "node:path";
import { traceOf } from "./lib/stream.mjs";
import { leaks, loadTask, resultsDir, ROOT, score, scoredFields, SETS } from "./lib/workspace.mjs";

const argv = process.argv.slice(2);
const SET = argv.includes("--set") ? argv[argv.indexOf("--set") + 1] : SETS[0];

// The run's tool calls, from its raw stream. Without the stream, the lookups already recorded stand.
function traceFor(r) {
  const file = r.stream_file && path.resolve(ROOT, r.stream_file);
  if (!file || !fs.existsSync(file)) return null;
  const events = [];
  for (const line of fs.readFileSync(file, "utf8").split("\n")) {
    if (!line.includes('"assistant"')) continue;
    try {
      events.push(JSON.parse(line));
    } catch {}
  }
  return traceOf(events);
}

let changed = 0;
let total = 0;
for (const isPrivate of [false, true]) {
  const file = path.join(resultsDir(SET, isPrivate), "runs.jsonl");
  if (!fs.existsSync(file)) continue;
  const out = fs.readFileSync(file, "utf8").split("\n").filter(Boolean).map((l) => {
    const r = JSON.parse(l);
    if (r.infra_error || !r.check_outputs) return l;
    total++;
    const trace = traceFor(r);
    const raw = Object.fromEntries(Object.entries(r.check_outputs).map(([k, v]) => [k, v.exit === 0]));
    const fields = trace ? scoredFields(raw, trace) : r.lookups?.some((x) => x.kind === "checkout") ? r.fields : raw;
    let { score: sc, pass, gates_failed } = score(fields, loadTask(r.task));
    if (r.timeout) [sc, pass] = [0, false];
    if (sc !== r.score) {
      changed++;
      console.log(`${r.task} ${r.config} r${r.round}: ${r.score} -> ${sc}`);
    }
    return JSON.stringify({ ...r, fields, score: sc, pass, gates_failed, false_success: r.claimed === "done" && !pass, lookups: trace ? leaks(trace) : r.lookups });
  });
  if (!argv.includes("--dry-run")) fs.writeFileSync(file, out.join("\n") + "\n");
}
console.log(`${changed} of ${total} rows changed${argv.includes("--dry-run") ? " (dry run)" : ""}`);
