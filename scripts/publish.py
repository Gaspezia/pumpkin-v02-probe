#!/usr/bin/env python3
"""PUBLISH <count> messages 'i epoch_ms' on channel probe at <rate>/s through one persistent redis-cli.
Usage: publish.py <redis-container> <count> <rate>"""
import subprocess, sys, time
c, n, rate = sys.argv[1], int(sys.argv[2]), float(sys.argv[3])
p = subprocess.Popen(["docker", "exec", "-i", c, "redis-cli"], stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, text=True)
t0 = time.time()
for i in range(1, n + 1):
    p.stdin.write(f'PUBLISH probe "{i} {int(time.time()*1000)}"\n'); p.stdin.flush()
    time.sleep(max(0, t0 + i / rate - time.time()))  # fixed schedule, no drift
p.stdin.close(); p.wait()
