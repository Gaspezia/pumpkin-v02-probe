#!/bin/bash
# Usage: run.sh <none|a|b> <stop|term>   Prints seconds to exit, exit code, last 5 log lines. No host port is published.
set -u
cd "$(dirname "$0")"
IMG=${PUMPKIN_IMAGE:-pumpkin-v02-probe:local}; V=$1; M=$2; N=stophang-$V-$M
WASM=target/wasm32-wasip2/release/stop_hang.wasm
R=$(mktemp -d); mkdir -p $R/plugins; cp pumpkin.toml $R/
[ "$V" = none ] || cp $WASM $R/plugins/
docker rm -f $N >/dev/null 2>&1
mkfifo $R/in; sleep 1000 > $R/in & HOLD=$!      # keeps stdin open
docker run -d -i --name $N --user $(id -u):$(id -g) -v $R:/pumpkin -e STOPHANG_VARIANT=$V --entrypoint sh $IMG -c 'exec pumpkin < /dev/stdin' >/dev/null
# -i container reads our fifo through docker attach
docker attach --no-stdin=false $N < $R/in >/dev/null 2>&1 & ATT=$!
for i in $(seq 120); do docker logs $N 2>&1 | grep -q "Server is now running" && break; sleep 1; done
sleep 2
S=$(date +%s.%N)
if [ $M = stop ]; then echo stop > $R/in & sleep 0.2; fi
if [ $M = term ]; then timeout 60 docker stop -t 30 $N >/dev/null & fi
for i in $(seq 1 400); do [ "$(docker inspect -f '{{.State.Running}}' $N)" = false ] && break; sleep 0.25; [ $i = 240 ] && [ $M = stop ] && break; done
E_=$(date +%s.%N)
RUN=$(docker inspect -f '{{.State.Running}}' $N)
echo "variant=$V method=$M running_at_end=$RUN seconds=$(awk "BEGIN{printf \"%.1f\", $E_ - $S}") exit=$(docker inspect -f '{{.State.ExitCode}}' $N)"
docker logs --tail 5 $N 2>&1 | sed 's/\x1b\[[0-9;]*m//g'
docker rm -f $N >/dev/null 2>&1; kill $HOLD $ATT 2>/dev/null; rm -rf $R
