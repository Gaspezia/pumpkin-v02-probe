#!/bin/bash
# Usage: NBOTS=10 DURATION=45 RATE=10 MSGS=100 scripts/run.sh [name]
# Starts the stack (compose project pumpkin-v02-probe), runs bots and publisher, writes
# results/raw/<name>.{log,-bots.json,-stats.csv} and prints the analysis. Stack stays up; stop it with
# `docker compose down`. .run/ holds generated state and secrets (gitignored) and is kept between runs.
set -euo pipefail
cd "$(dirname "$0")/.."
NBOTS=${NBOTS:-10}; DURATION=${DURATION:-45}; RATE=${RATE:-10}; MSGS=${MSGS:-100}
NAME=${1:-run-$(date +%Y%m%d-%H%M%S)}; PROXY_PORT=${PROXY_PORT:-15590}; export PROXY_PORT
export HOST_UID=${HOST_UID:-$(id -u)} HOST_GID=${HOST_GID:-$(id -g)}   # containers write .run/ as the calling user
WASM=${WASM:-target/wasm32-wasip2/release/async_probe.wasm}
mkdir -p results/raw .run/pumpkin/plugins .run/proxy
[ -f "$WASM" ] || { echo "build the plugin first: cargo build --release --target wasm32-wasip2"; exit 1; }
[ -d bots/node_modules ] || (cd bots && npm ci --no-audit --no-fund)
scripts/fetch-proxy.sh .run/proxy
SECRET=$(head -c 24 /dev/urandom | base64 | tr -dc 'A-Za-z0-9' | head -c 24)   # generated per run
printf %s "$SECRET" > .run/proxy/forwarding.secret
cp config/velocity.toml .run/proxy/velocity.toml
sed "s/@SECRET@/$SECRET/" config/pumpkin.toml > .run/pumpkin/pumpkin.toml
cp "$WASM" .run/pumpkin/plugins/async_probe.wasm
docker compose up -d --force-recreate
PUMPKIN=$(docker compose ps -q pumpkin); REDIS=$(docker compose ps -q redis)
for i in $(seq 180); do
  docker logs "$PUMPKIN" 2>&1 | grep -q "Server is now running" && break
  if [ "$i" = 180 ]; then echo "pumpkin not ready after 180 s; last 30 log lines:"; docker logs --tail 30 "$PUMPKIN" 2>&1; exit 1; fi
  sleep 1
done
sleep 12   # first clock window outside the scenario
R=results/raw/$NAME; : > $R-stats.csv
scripts/sample-stats.sh "$PUMPKIN" $R-stats.csv 15 & SAMPLER=$!
trap 'kill $SAMPLER 2>/dev/null || true' EXIT
echo "start $(date -u +%FT%TZ) NBOTS=$NBOTS DURATION=$DURATION RATE=$RATE MSGS=$MSGS"
( NOMOVE=1 CHAT=1 node bots/load.js "$NBOTS" "$DURATION" "$PROXY_PORT" pb > $R-bots.json 2>&1 ) & BOTS=$!
sleep 2
python3 scripts/publish.py "$REDIS" "$MSGS" "$RATE"
wait $BOTS
sleep 12
docker logs "$PUMPKIN" 2>&1 | sed 's/\x1b\[[0-9;]*m//g' > $R.log
kill $SAMPLER 2>/dev/null || true
python3 scripts/analyze.py "$NAME" "$MSGS" | tee results/$NAME.txt
