#!/bin/bash
# usage: heldout-check.sh existing | <module of src/heldout.rs>
# Run from the workspace. Temp files stay in the run's own directory.
set -u
R="$(cd .. && pwd)"
export TMPDIR="$R/tmp"
mkdir -p "$TMPDIR"
if [ "$1" = existing ]; then
  # the tests the project had before the change, by name. The touch makes
  # cargo build what is in the workspace: an export keeps its commit's file
  # times, which are older than the warmed build, and cargo would otherwise
  # run the cached base binary against sources it never compiled
  touch src/main.rs
  out=$(cargo test -- --exact $(cat heldout-existing.txt) 2>&1)
  echo "$out" | tail -n 40
  echo "$out" | grep -q "test result: ok. $(wc -l < heldout-existing.txt | tr -d ' ') passed; 0 failed"
else
  grep -q '^mod heldout;' src/main.rs || printf '\n#[cfg(test)]\nmod heldout;\n' >> src/main.rs
  out=$(cargo test "heldout::$1::" -- --test-threads=1 2>&1)
  echo "$out" | tail -n 60
  echo "$out" | grep -Eq 'test result: ok\. [1-9][0-9]* passed; 0 failed'
fi
