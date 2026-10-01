# code-bench

A small coding benchmark for choosing a Claude model and effort level. Each task is a real commit
from one of my own repos: the model starts from the commit's parent with the bug report or feature
request as its prompt, and the commit's tests, plus more written to probe the edges, are held out
and run after it finishes. The runs go through Claude Code (`claude -p`), so this measures the
model inside the tool I use, not the bare API.

It exists to answer one question each time a model ships: which model and effort should be the
daily driver, and which is worth its time on hard work?

## Results

Sonnet 5.5, Opus 5.5 and Fable 5.1, run on 2026-09-30 and 2026-10-01 with Claude Code 2.1.286.
Cost is the CLI's list-price `total_cost_usd`. A run passes when it meets every check of its task.

### Hard set: four Rust tasks, 10 checks each

Symptom-only prompts of about 150 words; the reference changes are 428 to about 3,000 lines.

| config | runs | passed | checks met | mean score | time, mean | cost, mean | said done but failed |
|---|---|---|---|---|---|---|---|
| Sonnet medium | 4 | 0 | 24/40 | 0.600 | 3.2 min | $0.52 | 4 |
| Sonnet high | 8 | 0 | 67/80 | 0.838 | 6.4 min | $1.16 | 8 |
| Sonnet xhigh | 4 | 1 | 37/40 | 0.925 | 22.6 min | $3.64 | 3 |
| Opus medium | 8 | 2 | 69/80 | 0.863 | 12.5 min | $3.14 | 6 |
| Opus high | 8 | 5 | 76/80 | 0.950 | 22.5 min | $5.76 | 3 |
| Fable low | 8 | 3 | 73/80 | 0.912 | 8.4 min | $4.88 | 5 |
| Fable medium | 4 | 1 | 36/40 | 0.900 | 14.0 min | $7.71 | 3 |

Score by task, one number per round:

| task | Sonnet medium | Sonnet high | Sonnet xhigh | Opus medium | Opus high | Fable low | Fable medium |
|---|---|---|---|---|---|---|---|
| catcher-big-feature | 0.6 | 0.9 / 0.8 | 1.0 | 0.6 / 0.8 | 1.0 / 0.8 | 0.9 / 1.0 | 0.8 |
| catcher-deep-bug | 0.5 | 0.9 / 0.9 | 0.9 | 1.0 / 1.0 | 0.9 / 1.0 | 1.0 / 1.0 | 0.9 |
| sidecar-hard (private) | 0.7 | 0.8 / 0.7 | 0.9 | 0.9 / 0.8 | 1.0 / 1.0 | 0.8 / 0.8 | 1.0 |
| squawk-hard | 0.6 | 0.9 / 0.8 | 0.9 | 0.9 / 0.9 | 1.0 / 0.9 | 0.9 / 0.9 | 0.9 |

### Easy set: four JS/TS tasks, 6 or 7 checks each

One-commit changes with prompts that state the rules and the interfaces.

| config | runs | passed | time, median | time, mean | cost, mean |
|---|---|---|---|---|---|
| Sonnet medium | 8 | 7 | 38 s | 46 s | $0.18 |
| Sonnet high | 8 | 8 | 70 s | 77 s | $0.28 |
| Opus medium | 4 | 4 | 2.9 min | 2.7 min | $0.89 |
| Opus high | 4 | 4 | 3.8 min | 3.6 min | $1.08 |
| Fable low | 8 | 7 | 83 s | 110 s | $1.27 |

Full tables are in [`results/hard/report.md`](results/hard/report.md) and
[`results/easy/report.md`](results/easy/report.md), and every run's numbers are in `scores.csv`
beside them.

### What they say

- **Easy work is near ceiling, so speed decides.** Every config passed almost everything. Sonnet
  medium is the fastest, Sonnet high was the only one to pass all eight runs, and Fable low
  averaged slower than both Sonnets on every task.
- **On hard work, effort buys score.** Sonnet met 24, then 35 and 37 of 40 checks at medium, high
  and xhigh (first round). Opus met 69 of 80 at medium and 76 at high.
- **Opus is worth it only at high.** Opus high passed 5 of 8 hard runs against Sonnet high's 0 of
  8 (Fisher p = 0.03). Opus medium passed 2 of 8 and met two more checks than Sonnet high in twice
  the time.
- **Fable low is the fast strong option.** It passed 3 of 8 against Opus high's 5 (p = 0.62) and
  met 73 checks against 76, in a third of the time. The two split by task: Opus high passed the
  state-machine task twice where Fable low scored 0.8 twice; Fable low passed catcher-deep-bug
  twice.
- **Sonnet xhigh costs its speed.** Two more checks than Sonnet high for 3.5 times the time; one
  run took 48 minutes.
- **A failed run still says it is done.** Of the 32 hard runs that missed a check, all 32 ended on
  `"status": "done"`.

What I do with it: Sonnet high as the daily driver, and Opus high (or Fable low when I can't wait)
for a piece that is hard in the sense below.

### Easy and hard

The split is how much the request leaves to work out, not size. The easy prompts state the rule
and roughly where it goes. The hard ones describe a symptom whose cause is somewhere else, several
causes at once, a state machine with racing paths, or a feature where the obvious reading is wrong.
A broad but mechanical change did not come out hard: an en/fr/de translation across 45 files was
passed twice by Sonnet at low effort, and was left out for that reason.

### Caveats

- **Small.** Four tasks a set, and one or two runs a cell. Among the top four hard configs only
  Opus high over Sonnet high is a difference the data supports.
- **Calibrated against Sonnet.** A hard task was kept only after Sonnet at low effort (not in the
  matrix) failed it twice. That selects for what a Sonnet model gets wrong.
- **My repos, my kind of work.** Rust terminal and desktop apps, a Next.js app, a browser
  extension's host. The hard set has no TypeScript task: Sonnet low passed all four tried for it.
- **Time includes builds.** Rust runs compile and test on a 10-core laptop, up to 12 at once.
  Running 12 at once did not slow them measurably (Sonnet high: 6.6 minutes at 12, 6.2 at 3).
- **One task's tests wait on the real clock** (sidecar-hard). Its rows were regraded one at a time
  on a quiet machine and came out the same.

## How a task is made

1. **A real change.** Pick a commit, or a run of commits, from a repo newer than the models'
   training data. The run starts from the parent with a one-commit history, so the fix isn't in
   its git objects.
2. **A prompt in the owner's words.** For the hard set, 40 to 150 words that say what is seen and
   wanted, and name no rule, file or interface.
3. **Held-out checks.** The commit's tests plus new ones for the edges the fix handles, split into
   named checks that each catch a different way of getting it wrong. At least one check runs the
   tests that already passed at base. `task.mjs verify` confirms every check fails or passes at
   base as declared, and that all pass at the fix.
4. **Calibration.** Run the weakest model twice. If it passes, the task is too easy: strip hints,
   widen the commits, or pick another change. Never by adding a demand the prompt doesn't make.
5. **A blind check.** A separate agent solves the task from the prompt and the code alone, then
   grades itself and classifies every calibration miss as a real defect or an unfair test, and
   repairs the unfair ones.

Scoring is partial credit over a task's checks, and pass when all are met. Files a task lists as
`protected` are put back to their base content before grading, so weakening an existing test
doesn't help. A run that reads the task's source checkout scores 0; a command that mentions a
remote (github.com, `gh`, `git clone`) only marks the row to be read by hand.

## Tasks

| task | set | repo | asks for | reference change |
|---|---|---|---|---|
| [catcher-big-feature](tasks/catcher-big-feature) | hard | [catcher](https://github.com/gautham-v/catcher) (Rust) | ordered lists that renumber on paste, Enter, delete and move, and Tab that nests | about 1,020 lines, 4 commits |
| [catcher-deep-bug](tasks/catcher-deep-bug) | hard | catcher | six symptoms where newly merged features meet | 428 lines, 7 files, 9 commits |
| [squawk-hard](tasks/squawk-hard) | hard | [squawk](https://github.com/gautham-v/squawk) (Rust) | a stubbed module finished: find the agent session, index its repo, turn spoken file names into mentions | about 3,000 lines, 11 files |
| sidecar-hard | hard | private (async Rust) | four symptoms of one state-machine bug | 600 lines, 7 files |
| [bridge-host-bug](tasks/bridge-host-bug) | easy | [firefox-agent-bridge](https://github.com/gautham-v/firefox-agent-bridge) (Node) | sub-agent steps shown in a chat panel | one commit |
| flashies-review-bug | easy | private (Next.js) | a status rule fixed in code, in SQL and in stored rows | one commit |
| flashies-shortcuts-feature | easy | private (Next.js) | a second key for each keyboard shortcut | one commit |
| flashies-practice-spec | easy | private (Next.js) | a shuffle setting that changes order and not selection | one commit |
| [bridge-extension-change](tasks/bridge-extension-change) | spare | firefox-agent-bridge | a per-client mode in a browser extension | one commit |

Each public task's directory has `task.json` (commits, prompt, checks), `hidden/` (the held-out
tests) and `notes.md` (why these commits, what the calibration runs got wrong, what the tests
assume). Tasks from private repos live under `private/`, which this repo doesn't track; their
scores are in `scores.csv`. Spare tasks are checked but not part of a set.

Publishing the held-out tests means a later model could have seen them. The fixes are in the
public repos' history anyway, so the tasks were never secret; if scores on a set start to look
too good, that set needs new tasks.

## Running it

Needs Node 22 or later (tested on 26), the [Claude Code](https://code.claude.com/docs) CLI signed
in, git, and a Rust toolchain for the Rust tasks. `repos.json` maps each repo to a local clone; change the paths
to where yours are. Built on macOS (workspaces are copy-on-write clones there; elsewhere they are
plain copies).

```sh
node task.mjs list
node task.mjs verify catcher-deep-bug     # every check as declared at base, all passing at the fix
node task.mjs warm catcher-deep-bug       # Rust: build the dependencies once; every run starts with a copy

node bench.mjs --set hard --configs sonnet:high,opus:high --rounds 2 --concurrency 12
node task.mjs regrade hard --only " r2"   # after a busy run: grade each saved diff again, alone
node rescore.mjs --set hard
node report.mjs --set hard                # results/hard/report.md and scores.csv
```

`bench.mjs` skips cells that already have a row, so the same command resumes a stopped run, and
adding a config or a round runs only what is missing.

### Testing a new model

A config is `model:effort`, where the model is `sonnet`, `opus`, `fable`, `haiku` or a full model
id, and the effort is `low`, `medium`, `high`, `xhigh` or `max`:

```sh
node bench.mjs --set easy --configs claude-new-model:medium,claude-new-model:high --rounds 2
node bench.mjs --set hard --configs claude-new-model:medium,claude-new-model:high --rounds 2
node report.mjs --set easy && node report.mjs --set hard
```

Then update the tables above. When an alias moves to a new model, change `MODELS` in `bench.mjs`,
or pass the full id so old and new rows stay apart.

## Layout

```
bench.mjs            the runner: schedules runs, grades them, appends rows
task.mjs             list, verify, warm, prepare, regrade
rescore.mjs          recompute scores from recorded check results
report.mjs           report.md and scores.csv for a set
lib/workspace.mjs    a task's workspace, grading and the lookup guard (the task format is its header)
lib/stream.mjs       per-run metrics from Claude Code's stream-json
tasks/<id>/          task.json, hidden/, notes.md
results/<set>/       runs.jsonl (one row a run), patches/ (each run's diff), report.md, scores.csv
repos.json           where each task's repo is cloned
```

`node --test test/*.test.mjs` runs the harness's own tests.

## License

MIT. The tasks quote code from the repos they come from, which carry their own licenses.
