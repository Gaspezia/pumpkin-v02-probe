#!/usr/bin/env python3
"""Usage: analyze.py <name> <expected_msgs> [raw_dir=results/raw]
Reads <name>.log (pumpkin log), <name>-bots.json, <name>-stats.csv and prints a summary."""
import re, sys, json, statistics as st
name, expected = sys.argv[1], int(sys.argv[2])
d = sys.argv[3] if len(sys.argv) > 3 else 'results/raw'
L = open(f'{d}/{name}.log').read().splitlines()
msgs = [(int(m[1]), int(m[2])) for m in (re.search(r'MSG id=(\d+) recv=\d+ sent=\d+ lat=(-?\d+)ms', l) for l in L) if m]
ids = {i for i, _ in msgs}; lat = sorted(v for _, v in msgs)
print(f'messages: received {len(ids)} unique / sent {expected} ({len(msgs)} log lines)')
if lat:
    print(f'latency ms: median={st.median(lat)} p99={lat[max(0, int(len(lat)*.99)-1)]} max={lat[-1]} min={lat[0]}')
ck = [re.search(r'CLOCK t=(\d+) avg=([\d.]+)ms max=([\d.]+)ms n=(\d+) over200=(\d+) listening=(\w+) received=(\d+)', l) for l in L]
ck = [c.groups() for c in ck if c]
if ck:
    av = [float(c[1]) for c in ck]
    print(f'clock windows(10s): {len(ck)}, mean of avg={st.mean(av):.2f}ms min={min(av):.1f} max={max(av):.1f}; '
          f'max tick interval={max(float(c[2]) for c in ck):.1f}ms; ticks >200ms={sum(int(c[4]) for c in ck)}; '
          f'windows with listening=false: {sum(c[5] != "true" for c in ck)}')
else:
    print('clock windows: none (no CLOCK line)')
j = sum('EVENT join' in l for l in L); c = sum('EVENT chat' in l for l in L)
print(f'logged events: joins={j} chats={c}')
print('log errors/panics:', [l[:140] for l in L if (re.search(r'panic|trap|deadlock', l, re.I) or ' ERROR ' in l)][:5] or 'none')
try:
    b = json.loads(open(f'{d}/{name}-bots.json').read().strip().splitlines()[-1])
    print(f"bots: n={b['n']} spawned={b['spawned']} kicked={b['kicked']} errors={b['errors']} connected_at_end={b['stillConnected']}")
except Exception as e:
    print('bots: no summary', e)
def mib(s):
    v, u = re.match(r'([\d.]+)\s*([KMG]i?B)', s).groups(); return float(v) * {'K': 1/1024, 'M': 1, 'G': 1024}[u[0]]
try:
    rows = [r.split(';') for r in open(f'{d}/{name}-stats.csv').read().splitlines() if r.count(';') == 2]
    S = [(int(t), mib(m.split('/')[0].strip()), float(c.strip('%'))) for t, m, c in rows]
    t0 = S[0][0]
    def at(s): return min(S, key=lambda x: abs(x[0] - t0 - s))
    pts = [S[0]] + [at(k * 300) for k in range(1, int((S[-1][0] - t0) // 300) + 1)] + [S[-1]]
    print('RAM/CPU (container, docker stats; sample interval ~17 s) at t=start, every 5 min, end:')
    for t, m, c in pts: print(f'  t={t-t0:5d}s  {m:7.1f} MiB  {c:5.1f} %')
    w = [x for x in S if x[0] - t0 >= 300]  # after 5 min warm-up
    if len(w) > 10:
        xs = [(x[0] - t0) / 600 for x in w]; ys = [x[1] for x in w]
        mx, my = st.mean(xs), st.mean(ys)
        print(f'RAM slope after 5 min warm-up: {sum((x-mx)*(y-my) for x,y in zip(xs,ys))/sum((x-mx)**2 for x in xs):+.2f} MiB per 10 min ({len(w)} samples)')
    else:
        print('RAM slope: run too short')
    print(f'CPU mean={st.mean(x[2] for x in S):.1f}% max={max(x[2] for x in S):.1f}%')
except Exception as e:
    print('stats: none', e)
