#!/bin/bash
# Held-out checks.
#   run.sh existing   every test that existed before the change still passes
#   run.sh <module>   the tests in src/app/heldout_lists.rs under that module
#
# The app's tests open a real session, which reads and writes a settings note
# under $HOME and scratch vaults under $TMPDIR; both are pointed inside this
# run's own directory so nothing outside it is touched and parallel graders
# cannot meet.
set -u
if [ -z "${CARGO_TARGET_DIR:-}" ]; then
  echo "CARGO_TARGET_DIR is not set"
  exit 2
fi
root="$(dirname "$CARGO_TARGET_DIR")"
export RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
export HOME="$root/check-home"
export TMPDIR="$root/check-tmp"
unset CATCHER_DIR
mkdir -p "$HOME" "$TMPDIR"

# the held-out module hangs off the app module, wherever that lives
app=src/app.rs
[ -f "$app" ] || app=src/app/mod.rs
grep -q '^mod heldout_lists;' "$app" || printf '\n#[cfg(test)]\nmod heldout_lists;\n' >> "$app"

what="${1:?which check}"
log="$TMPDIR/check-$what.log"

if [ "$what" = existing ]; then
  cargo test -- --skip heldout_lists > "$log" 2>&1
  sed -n 's/^.*test \(.*\) \.\.\. ok$/\1/p' "$log" | sort -u > "$TMPDIR/ok-tests.txt"
  missing="$(sort -u heldout/base_tests.txt | comm -23 - "$TMPDIR/ok-tests.txt")"
  if [ -n "$missing" ]; then
    grep -v '^test .* \.\.\. ok$' "$log" | tail -n 25
    echo "existing tests that did not pass ($(printf '%s\n' "$missing" | wc -l | tr -d ' ')):"
    printf '%s\n' "$missing" | head -n 15
    exit 1
  fi
  echo "all $(wc -l < heldout/base_tests.txt | tr -d ' ') existing tests pass"
  exit 0
fi

cargo test "app::heldout_lists::$what::" > "$log" 2>&1
rc=$?
# the terminal title the app sets is noise here
perl -pe 's/\e\]0;[^\a]*\a//g' "$log" | grep -v '^$' | grep -v '^note: run with' | grep -v '^test .* \.\.\. ok$' | tail -n 40
[ $rc -eq 0 ] && grep -Eq 'test result: ok\. [1-9][0-9]* passed' "$log"
