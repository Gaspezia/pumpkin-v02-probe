#!/bin/bash
# Checks out upstream Pumpkin master (eda69440e) merged with the head of PR #3962 (0740d3f6) into <dir>.
# Used by the Dockerfile (server build) and for the plugin's SDK path dependency (./pumpkin-src).
# Expected tree hash of the result: 92b6ca1617ae5d02d4fbbac0ed6f39d2c93b9f74
set -euo pipefail
D=${1:-pumpkin-src}
MASTER=eda69440e6369c5f8a05c3f69fc809c40dab93a6
PR_HEAD=0740d3f624d94e35c0e8a16b0839d895f9670a6d
[ -d "$D/.git" ] && { echo "$D exists"; exit 0; }
mkdir -p "$D"; cd "$D"
git init -q .
git remote add origin https://github.com/Pumpkin-MC/Pumpkin.git
git fetch -q origin "$MASTER"
git checkout -q "$MASTER"
git fetch -q https://github.com/Phoenixxo/Pumpkin.git "$PR_HEAD"
git -c user.name=build -c user.email=build@localhost merge -q --no-edit "$PR_HEAD"
echo "tree $(git rev-parse 'HEAD^{tree}')"
