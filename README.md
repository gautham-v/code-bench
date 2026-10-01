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
Cost is the CLI's list-price `total_cost_usd`.

The benchmark keeps only what current models have failed. Four Rust tasks are left, each with a
symptom-only prompt of about 150 words and a reference change of 428 to about 3,000 lines. Of
their 40 checks, 22 were met by every one of the 44 runs below; those are **gates**, which a run
must still meet to pass but which earn no score. The score is the share of the other 18, the
checks at least one run missed. A run passes when it meets every check.

| config | runs | passed | scored checks met | mean score | time, mean | cost, mean | said done but failed |
|---|---|---|---|---|---|---|---|
| Sonnet medium | 4 | 0 | 2/18 | 0.083 | 3.2 min | $0.52 | 4 |
| Sonnet high | 8 | 0 | 23/36 | 0.585 | 6.4 min | $1.16 | 8 |
| Sonnet xhigh | 4 | 1 | 15/18 | 0.804 | 22.6 min | $3.64 | 3 |
| Opus medium | 8 | 2 | 25/36 | 0.688 | 12.5 min | $3.14 | 6 |
| Opus high | 8 | 5 | 32/36 | 0.902 | 22.5 min | $5.76 | 3 |
| Fable low | 8 | 3 | 29/36 | 0.750 | 8.4 min | $4.88 | 5 |
| Fable medium | 4 | 1 | 14/18 | 0.804 | 14.0 min | $7.71 | 3 |

Score by task, one number per round:

| task | scored checks | Sonnet medium | Sonnet high | Sonnet xhigh | Opus medium | Opus high | Fable low | Fable medium |
|---|---|---|---|---|---|---|---|---|
| catcher-big-feature | 6 | 0.33 | 0.83 / 0.67 | 1.0 | 0.33 / 0.67 | 1.0 / 0.67 | 0.83 / 1.0 | 0.67 |
| catcher-deep-bug | 5 | 0 | 0.8 / 0.8 | 0.8 | 1.0 / 1.0 | 0.8 / 1.0 | 1.0 / 1.0 | 0.8 |
| sidecar-hard (private) | 3 | 0 | 0.33 / 0 | 0.67 | 0.67 / 0.33 | 1.0 / 1.0 | 0.33 / 0.33 | 1.0 |
| squawk-hard | 4 | 0 | 0.75 / 0.5 | 0.75 | 0.75 / 0.75 | 1.0 / 0.75 | 0.75 / 0.75 | 0.75 |

The full tables, with every check and every run's time, are in
[`results/hard/report.md`](results/hard/report.md), and each run's numbers are in
[`scores.csv`](results/hard/scores.csv) beside it.

### What they say

- **Effort buys score.** Sonnet met 2, then 13 and 15 of the 18 scored checks at medium, high and
  xhigh (first round). Opus met 25 of 36 at medium and 32 at high.
- **Opus is worth it only at high.** Opus high passed 5 of 8 runs against Sonnet high's 0 of 8
  (Fisher p = 0.03). Opus medium passed 2 of 8 and met two more checks than Sonnet high in twice
  the time.
- **Fable low is the fast strong option.** It passed 3 of 8 against Opus high's 5 (p = 0.62) and
  met 29 checks against 32, in a third of the time. The two split by task: Opus high passed the
  state-machine task twice where Fable low scored 0.33 twice; Fable low passed catcher-deep-bug
  twice.
- **Sonnet xhigh costs its speed.** Two more checks than Sonnet high for 3.5 times the time; one
  run took 48 minutes.
- **A failed run still says it is done.** Of the 32 runs that missed a check, all 32 ended on
  `"status": "done"`.

What I do with it: Sonnet high as the daily driver, and Opus high (or Fable low when I can't wait)
for a piece that is hard in the sense below.

### Retired: the easy set

The first draft was four one-commit JS/TS tasks whose prompts stated the rules and the interfaces.
Every config passed nearly everything, so it could only rank them on speed, and it could not show
a later model doing better. It was removed on 2026-10-01, with two spare tasks that were never
part of a set. What it measured:

| config | runs | passed | time, median | time, mean | cost, mean |
|---|---|---|---|---|---|
| Sonnet medium | 8 | 7 | 38 s | 46 s | $0.18 |
| Sonnet high | 8 | 8 | 70 s | 77 s | $0.28 |
| Opus medium | 4 | 4 | 2.9 min | 2.7 min | $0.89 |
| Opus high | 4 | 4 | 3.8 min | 3.6 min | $1.08 |
| Fable low | 8 | 7 | 83 s | 110 s | $1.27 |

On easy work speed decides: Sonnet medium is the fastest, Sonnet high was the only one to pass all
eight runs, and Fable low averaged slower than both Sonnets on every task. The tasks and rows are
in this repo's history at `faca80c`.

### Easy and hard

The split is how much the request leaves to work out, not size. The easy prompts stated the rule
and roughly where it goes. The hard ones describe a symptom whose cause is somewhere else, several
causes at once, a state machine with racing paths, or a feature where the obvious reading is wrong.
A broad but mechanical change did not come out hard: an en/fr/de translation across 45 files was
passed twice by Sonnet at low effort.

### Caveats

- **Small.** Four tasks, 18 scored checks, and one or two runs a cell. Among the top four configs
  only Opus high over Sonnet high is a difference the data supports.
- **Little room above the best config.** Opus high met 32 of its 36 scored checks, so a model
  better than it can show at most four more. The set will need harder tasks to measure that model.
- **The scored checks were picked by these runs.** A check is scored because one of the 44 runs
  missed it, so the configs above look a little worse on them than a fresh run of the same configs
  would. A new model's first runs carry no such tilt.
- **Calibrated against Sonnet.** A task was kept only after Sonnet at low effort (not in the
  matrix) failed it twice. That selects for what a Sonnet model gets wrong.
- **My repos, my kind of work.** Rust terminal and desktop apps. There is no TypeScript task:
  Sonnet low passed all four tried for the hard set.
- **Time includes builds.** Runs compile and test on a 10-core laptop, up to 12 at once. Running
  12 at once did not slow them measurably (Sonnet high: 6.6 minutes at 12, 6.2 at 3).
- **One task's tests wait on the real clock** (sidecar-hard). Its rows were regraded one at a time
  on a quiet machine and came out the same.

## How a task is made

1. **A real change.** Pick a commit, or a run of commits, from a repo newer than the models'
   training data. The run starts from the parent with a one-commit history, so the fix isn't in
   its git objects.
2. **A prompt in the owner's words.** 40 to 150 words that say what is seen and wanted, and name
   no rule, file or interface.
3. **Held-out checks.** The commit's tests plus new ones for the edges the fix handles, split into
   named checks that each catch a different way of getting it wrong. At least one check runs the
   tests that already passed at base. `task.mjs verify` confirms every check fails or passes at
   base as declared, and that all pass at the fix.
4. **Calibration.** Run the weakest model twice. If it passes, the task is too easy: strip hints,
   widen the commits, or pick another change. Never by adding a demand the prompt doesn't make.
5. **A blind check.** A separate agent solves the task from the prompt and the code alone, then
   grades itself and classifies every calibration miss as a real defect or an unfair test, and
   repairs the unfair ones.
6. **Gates.** Once models have run it, a check every run met is marked `"gate": true` in
   `task.json`. It still has to pass, and it stops counting toward the score.

Scoring is partial credit over a task's scored checks, and pass when every check is met. Files a
task lists as `protected` are put back to their base content before grading, so weakening an
existing test doesn't help. A run that reads the task's source checkout scores 0; a command that
mentions a remote (github.com, `gh`, `git clone`) only marks the row to be read by hand.

## Tasks

| task | repo | asks for | reference change | scored checks |
|---|---|---|---|---|
| [catcher-big-feature](tasks/catcher-big-feature) | [catcher](https://github.com/gautham-v/catcher) (Rust) | ordered lists that renumber on paste, Enter, delete and move, and Tab that nests | about 1,020 lines, 4 commits | 6 of 10 |
| [catcher-deep-bug](tasks/catcher-deep-bug) | catcher | six symptoms where newly merged features meet | 428 lines, 7 files, 9 commits | 5 of 10 |
| [squawk-hard](tasks/squawk-hard) | [squawk](https://github.com/gautham-v/squawk) (Rust) | a stubbed module finished: find the agent session, index its repo, turn spoken file names into mentions | about 3,000 lines, 11 files | 4 of 10 |
| sidecar-hard | private (async Rust) | four symptoms of one state-machine bug | 600 lines, 7 files | 3 of 10 |

Each public task's directory has `task.json` (commits, prompt, checks), `hidden/` (the held-out
tests) and `notes.md` (why these commits, what the calibration runs got wrong, what the tests
assume). The task from a private repo lives under `private/`, which this repo doesn't track; its
scores are in `scores.csv`.

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

node bench.mjs --configs sonnet:high,opus:high --rounds 2 --concurrency 12
node task.mjs regrade hard --only " r2"   # after a busy run: grade each saved diff again, alone
node rescore.mjs
node report.mjs                           # results/hard/report.md and scores.csv
```

`bench.mjs` skips cells that already have a row, so the same command resumes a stopped run, and
adding a config or a round runs only what is missing.

### Testing a new model

A config is `model:effort`, where the model is `sonnet`, `opus`, `fable`, `haiku` or a full model
id, and the effort is `low`, `medium`, `high`, `xhigh` or `max`:

```sh
node bench.mjs --configs claude-new-model:medium,claude-new-model:high --rounds 2
node report.mjs
```

Then update the tables above. The gates stay as they are: a new model that misses one shows up in
the report's "gates failed" column and fails that run. When an alias moves to a new model, change `MODELS` in `bench.mjs`,
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
results/hard/        runs.jsonl (one row a run), patches/ (each run's diff), report.md, scores.csv
repos.json           where each task's repo is cloned
```

`node --test test/*.test.mjs` runs the harness's own tests.

## License

MIT. The tasks quote code from the repos they come from, which carry their own licenses.
