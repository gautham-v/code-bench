#!/usr/bin/env node
// Summarizes a set's runs into results/<set>/report.md and results/<set>/scores.csv. It reads the
// public rows and, when they are present, the private tasks' rows, so the two files cover every
// task with numbers only: scores.csv is what a reader without the private tasks can check the
// tables against. Rows that failed for infrastructure reasons are left out; timeouts count as 0.
//
//   node report.mjs [--set hard]

import fs from "node:fs";
import path from "node:path";
import { listTasks, resultsDir, SETS } from "./lib/workspace.mjs";

const argv = process.argv.slice(2);
const SET = argv.includes("--set") ? argv[argv.indexOf("--set") + 1] : SETS[0];

const rows = [false, true]
  .flatMap((p) => {
    const f = path.join(resultsDir(SET, p), "runs.jsonl");
    return fs.existsSync(f) ? fs.readFileSync(f, "utf8").split("\n").filter(Boolean).map((l) => JSON.parse(l)) : [];
  })
  .filter((r) => !r.infra_error);
const isPrivate = Object.fromEntries(listTasks().map((t) => [t.id, t.private]));
// A task's gates: checks every model measured so far has met. They are needed to pass and earn no score.
const gates = Object.fromEntries(listTasks().map((t) => [t.id, new Set(t.checks.filter((c) => c.gate).map((c) => c.name))]));
const isGate = (t, k) => gates[t]?.has(k) ?? false;

const MODEL_ORDER = ["haiku", "sonnet", "opus", "fable"];
const EFFORT_ORDER = ["default", "low", "medium", "high", "xhigh", "max"];
// Known models in a fixed order, then anything new, by name.
const order = (c) => {
  const m = MODEL_ORDER.findIndex((x) => c.includes(x));
  return [m < 0 ? MODEL_ORDER.length : m, c.split("/")[0], EFFORT_ORDER.indexOf(c.split("/")[1])];
};
const byOrder = (a, b) => {
  const [x, y] = [order(a), order(b)];
  return x[0] - y[0] || x[1].localeCompare(y[1]) || x[2] - y[2];
};
const configs = [...new Set(rows.map((r) => r.config))].sort(byOrder);
const tasks = [...new Set(rows.map((r) => r.task))].sort();
const short = (c) => c.replace(/^claude-/, "").replace(/-\d+-\d+(-\d+)?\//, " ");
const mean = (xs) => (xs.length ? xs.reduce((a, b) => a + b, 0) / xs.length : NaN);
const median = (xs) => {
  const s = xs.filter(Number.isFinite).sort((a, b) => a - b);
  return s.length ? (s[Math.floor((s.length - 1) / 2)] + s[Math.ceil((s.length - 1) / 2)]) / 2 : NaN;
};
const f = (v, d = 2) => (Number.isFinite(v) ? v.toFixed(d) : "–");
const dur = (s) => (!Number.isFinite(s) ? "–" : s < 120 ? `${s.toFixed(0)} s` : `${(s / 60).toFixed(1)} min`);
const checks = (rs) => rs.flatMap((r) => Object.entries(r.fields ?? {}).filter(([k]) => !isGate(r.task, k)).map(([, v]) => v));
const out = (r) => (r.model_usage ? Object.values(r.model_usage).reduce((a, u) => a + (u.outputTokens ?? 0), 0) : r.usage?.output_tokens) ?? NaN;
const name = (t) => `${t}${isPrivate[t] ? " (private)" : ""}`;

let md = `# ${SET}: ${rows.length} runs\n\nCost is the CLI's list-price \`total_cost_usd\`. A run passes when it meets every check of its task, gates included; score is the share of its scored checks met. A gate is a check every model measured so far has met, so it earns nothing. Tasks marked private come from repos that aren't public, so their prompts, tests and diffs aren't in this repo; their numbers are.\n`;

md += `\n## Per config\n\n| config | runs | passed | scored checks met | mean score | gates failed | said done but failed | timeouts | wall, mean | wall, median | cost, mean | tool calls, median | output tokens, median | lines +/−, median |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n`;
for (const c of configs) {
  const rs = rows.filter((r) => r.config === c);
  const ch = checks(rs);
  md += `| ${short(c)} | ${rs.length} | ${rs.filter((r) => r.pass).length} | ${ch.filter(Boolean).length}/${ch.length} | ${f(mean(rs.map((r) => r.score)), 3)} | ${rs.reduce((a, r) => a + (r.gates_failed ?? 0), 0)} | ${rs.filter((r) => r.false_success).length} | ${rs.filter((r) => r.timeout).length} | ${dur(mean(rs.map((r) => r.wall_ms / 1000)))} | ${dur(median(rs.map((r) => r.wall_ms / 1000)))} | $${f(mean(rs.map((r) => r.cost_usd ?? NaN)))} | ${f(median(rs.map((r) => r.tool_calls)), 0)} | ${f(median(rs.map(out)), 0)} | +${f(median(rs.map((r) => r.lines_added)), 0)} / −${f(median(rs.map((r) => r.lines_removed)), 0)} |\n`;
}

const cell = (t, c, fn) => {
  const rs = rows.filter((r) => r.task === t && r.config === c).sort((a, b) => a.round - b.round);
  return rs.length ? rs.map(fn).join(" / ") : "";
};
md += `\n## Score by task and round\n\n| task | ${configs.map(short).join(" | ")} |\n|---|${configs.map(() => "---").join("|")}|\n`;
for (const t of tasks) md += `| ${name(t)} | ${configs.map((c) => cell(t, c, (r) => f(r.score, 2).replace(/\.?0+$/, "") || "0")).join(" | ")} |\n`;

md += `\n## Wall time by task and round\n\n| task | ${configs.map(short).join(" | ")} |\n|---|${configs.map(() => "---").join("|")}|\n`;
for (const t of tasks) md += `| ${name(t)} | ${configs.map((c) => cell(t, c, (r) => dur(r.wall_ms / 1000))).join(" | ")} |\n`;

md += `\n## Checks, by how many runs met them\n\n| task | check | kind | ${configs.map(short).join(" | ")} |\n|---|---|---|${configs.map(() => "---").join("|")}|\n`;
for (const t of tasks) {
  for (const k of [...new Set(rows.filter((r) => r.task === t).flatMap((r) => Object.keys(r.fields ?? {})))]) {
    md += `| ${name(t)} | ${k} | ${isGate(t, k) ? "gate" : "scored"} | ${configs
      .map((c) => {
        const rs = rows.filter((r) => r.task === t && r.config === c && k in (r.fields ?? {}));
        return rs.length ? `${rs.filter((r) => r.fields[k]).length}/${rs.length}` : "";
      })
      .join(" | ")} |\n`;
  }
}

const marked = rows.filter((r) => r.lookups?.length);
if (marked.length) {
  md += `\n## Rows the lookup guard marked\n\nA \`remote\` mark is a pattern in a command (github.com, gh, git clone) and is read by hand; it doesn't change the score. A \`checkout\` mark means the run read a source checkout and scored 0.\n\n| task | config | round | marks |\n|---|---|---|---|\n`;
  for (const r of marked) md += `| ${name(r.task)} | ${short(r.config)} | ${r.round} | ${r.lookups.map((l) => l.kind).join(", ")} |\n`;
}

const csv = ["task,private,config,round,score,pass,scored_met,scored,gates_failed,claimed,wall_s,cost_usd,tool_calls,turns,output_tokens,lines_added,lines_removed,timeout"];
for (const r of [...rows].sort((a, b) => a.task.localeCompare(b.task) || byOrder(a.config, b.config) || a.round - b.round)) {
  const v = checks([r]);
  csv.push([r.task, isPrivate[r.task] ? 1 : 0, r.config, r.round, r.score, r.pass ? 1 : 0, v.filter(Boolean).length, v.length, r.gates_failed ?? 0, r.claimed ?? "", (r.wall_ms / 1000).toFixed(0), (r.cost_usd ?? 0).toFixed(3), r.tool_calls, r.turns ?? "", out(r) || "", r.lines_added, r.lines_removed, r.timeout ? 1 : 0].join(","));
}

const dir = resultsDir(SET);
fs.writeFileSync(path.join(dir, "report.md"), md);
fs.writeFileSync(path.join(dir, "scores.csv"), csv.join("\n") + "\n");
console.log(md);
console.log(`wrote results/${SET}/report.md and scores.csv`);
