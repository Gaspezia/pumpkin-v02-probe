#!/bin/bash
# Usage: sample-stats.sh <container> <out.csv> [interval=15]. Appends "epoch;mem;cpu" until killed.
C=$1; O=$2; I=${3:-15}
while true; do
  echo "$(date +%s);$(docker stats --no-stream --format '{{.MemUsage}};{{.CPUPerc}}' "$C")" >> "$O"
  sleep "$I"
done
