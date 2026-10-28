# code-bench

Public MIT coding benchmark for choosing a Claude model and effort level. `README.md` holds the results tables, how a task is made, and "Testing a new model"; read it first. The benchmark is rerun on each new model release.

## Rules

- `private/` is a nested repo (pushed to the private `gautham-v/code-bench-private`) holding tasks from private repos. Never commit its task material, diffs or transcripts to this repo; only its scores are public. Describe those tasks generically (for example "async Rust state machine") in anything public.
- Raw transcripts (`results/*/streams/`) are never committed: they carry the user's Claude config. They are gitignored; keep them that way.
- Rust runs need their own `CARGO_TARGET_DIR` (the machine's `~/.cargo/config.toml` sets one shared target dir, which would make runs collide) and a warmed cache from `task.mjs warm` (`.cache/`, gitignored, a few GB). The task's `task.json` env sets `CARGO_TARGET_DIR` to `{root}/target`.
- Calibrate new tasks against a config that is not in the matrix (Sonnet low). After a busy concurrent run, regrade serially with `task.mjs regrade` before reporting; one task's tests wait on the real clock.
- Optimize for speed and quality, not cost; the runs use a subscription. Report time alongside score.
- Social or blog copy about results should carry takeaways, not numbers the chart already shows, with Rust in the first line.
