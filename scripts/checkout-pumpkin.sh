#!/bin/bash
# Checks out upstream Pumpkin master (eda69440e) merged with the head of PR #3962 (0740d3f6) into <dir>.
# Used by the Dockerfile (server build) and for the plugin's SDK path dependency (./pumpkin-src).
# Fails if the resulting tree is not the one the results were produced with.
set -euo pipefail
D=${1:-pumpkin-src}
MASTER=eda69440e6369c5f8a05c3f69fc809c40dab93a6
PR_HEAD=0740d3f624d94e35c0e8a16b0839d895f9670a6d
TREE=92b6ca1617ae5d02d4fbbac0ed6f39d2c93b9f74
check() {
  got=$(git -C "$D" rev-parse 'HEAD^{tree}')
  [ "$got" = "$TREE" ] || { echo "tree mismatch in $D: got $got, expected $TREE" >&2; exit 1; }
  echo "tree $got"
}
if [ -d "$D/.git" ]; then echo "$D exists"; check; exit 0; fi
mkdir -p "$D"; cd "$D"
git init -q .
git remote add origin https://github.com/Pumpkin-MC/Pumpkin.git
git fetch -q origin "$MASTER"
git checkout -q "$MASTER"
# PR author's fork first; if it is gone, the PR ref upstream (works only while the PR head is still $PR_HEAD)
git fetch -q https://github.com/Phoenixxo/Pumpkin.git "$PR_HEAD" 2>/dev/null \
  || { git fetch -q origin refs/pull/3962/head && git cat-file -e "$PR_HEAD^{commit}"; }
git -c user.name=build -c user.email=build@localhost merge -q --no-edit "$PR_HEAD"
cd - >/dev/null
check
