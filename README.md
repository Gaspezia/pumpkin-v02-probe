# pumpkin-v02-probe

Minimal reproduction for a test report on Pumpkin-MC/Pumpkin PR #3962 (WASM plugin API v0.2).

It checks one thing: a **long-lived Redis subscriber running as a background future in a v0.2 plugin**
(`wit_bindgen::spawn_local` started from `on_load`, WASI p3 TCP socket), while the server keeps ticking
and firing plugin events. The probe plugin logs:

- `MSG`: every Redis pub/sub message with its latency (publisher epoch ms vs receive epoch ms),
- `CLOCK`: tick interval stats from a 1-tick repeating task, per 10 s window,
- `EVENT join` / `EVENT chat`: player join and chat events delivered to the plugin.

Bots (Mineflayer, through Velocity + ViaVersion) join, chat every 3 s and hit the nearest player every 1.5 s while messages are published.

## Versions

| Component | Version |
|---|---|
| Pumpkin server | upstream master `eda69440e6369c5f8a05c3f69fc809c40dab93a6` (`0.2.2+26.3-26.60`) merged with PR #3962 head `0740d3f624d94e35c0e8a16b0839d895f9670a6d`, `--features plugin-v0_2` |
| Plugin SDK | `pumpkin-plugin-api-v0_2` from the same merged tree (path dependency on `pumpkin-src/`) |
| wit-bindgen | 0.63 (SDK + plugin, feature `async-spawn`); 0.62 is also in the tree, pulled by `wasip3` |
| wasip3 | 0.9 from crates.io (WASI 0.3.0, matches the host) |
| Rust | 1.98 (Docker), 1.99 locally, target `wasm32-wasip2` |
| Proxy | Velocity 4.3.0 build 44, ViaVersion + ViaBackwards 5.12.2 (Mineflayer 4.39 speaks 26.1 at most, Pumpkin speaks 26.3) |
| Bots | Mineflayer 4.39.0, Node 22 |

The merge is redone by `scripts/checkout-pumpkin.sh`. Its tree hash is `92b6ca1617ae5d02d4fbbac0ed6f39d2c93b9f74`,
identical to the local merge commit used for the results below.

## Build and run

```bash
scripts/checkout-pumpkin.sh                       # ./pumpkin-src = master + PR head (about 30 s)
cargo build --release --target wasm32-wasip2      # target/wasm32-wasip2/release/async_probe.wasm
docker compose build pumpkin                      # server image, about 30 min; or set PUMPKIN_IMAGE
NBOTS=10 DURATION=45 RATE=10 MSGS=100 scripts/run.sh short-45s
NBOTS=20 DURATION=1800 RATE=2 MSGS=3600 scripts/run.sh soak-30min
docker compose down
```

`scripts/run.sh <name>` generates the Velocity forwarding secret, writes `.run/` (gitignored), starts the stack
(compose project `pumpkin-v02-probe`), runs the bots and the publisher, samples `docker stats` of the Pumpkin
container every 15 s, and writes `results/raw/<name>.*` plus `results/<name>.txt`. Plugin permissions
(`network.tcp.connect`, `network.dns`, `sys.env`) are pre-approved in `config/pumpkin.toml`; telemetry and the
update checker are off. Pumpkin runs with `cpus: 1.0`, `mem_limit: 4g`. Needs Docker, Node 22, Python 3 and a
Rust toolchain with the `wasm32-wasip2` target.

Host ports: only `127.0.0.1:${PROXY_PORT:-15590}` (Velocity). Redis and Pumpkin publish nothing.
`.run/` persists between runs (the generated world is reused); delete it for a fresh world. The containers run as
your uid/gid (`HOST_UID`/`HOST_GID`, set by `run.sh`), so `.run/` stays owned by you. Without `run.sh`, export them
before `docker compose`.

## Pitfalls we hit

- `spawn_local` from `on_load` works. The subscriber connects at plugin load and stays suspended on the socket;
  ticks, scheduled tasks and events keep running.
- Running the never-ending async socket future (`listen_p3`) inside a `schedule_delayed_task` closure
  (`PROBE_VARIANT=a2`), or a blocking `std::net` read loop in a task (`b`), freezes the whole plugin instance:
  no more `CLOCK` lines from the repeating task and no `EVENT join/chat`, although messages are still received.
  The server itself keeps running and bots stay connected. Seen in one 45 s run per variant (logs not in this repo),
  cause not investigated.
- The plugin must be built against the SDK from the same tree as the server. The SDK from the PR head alone
  (`0740d3f6`, base `1ec04f660`) does not load on a server built from master `eda69440e` + the PR:
  `Plugin was built against a different iteration of the API: component imports instance
  `pumpkin:plugin/context@0.2.0`, but a matching implementation was not found in the linker`.
  Master changed `bedrock-packets.wit`, `event.wit` and `java-packets.wit` after the PR's base.
- Plugin permissions are pre-approved in `[plugins] allowed_permissions` of `pumpkin.toml` (not tested without).
- WIT/async runtime versions: with `wasip3` 0.9 (wit-bindgen 0.62) next to the SDK's 0.63 the probe built and
  worked in one 45 s run (`results/unpatched-scratch-45s.txt`). We earlier carried a vendored `wasip3` bumped to 0.63;
  it is not needed for this probe and is not in this repo.

## Results

Pumpkin `0.2.2+26.3-26.60` (master `eda69440e` + PR head `0740d3f6`), 1 CPU / 4 GiB container, bots `NOMOVE=1 CHAT=1`.
All committed results below were produced with commit `2c17ccb` (the Redis channel was then named `gzprobe`, now `probe`).
Files are in `results/` (summaries) and `results/raw/` (logs, `docker stats` samples, bot summaries).

| Files | Configuration |
|---|---|
| `short-45s.*` | Repo build: plugin from `./pumpkin-src` SDK, unpatched `wasip3` 0.9 (wit-bindgen 0.62 + 0.63), 10 bots, 45 s, 100 msgs at 10/s |
| `unpatched-scratch-45s.*` | Same configuration built from a scratch copy of the sources (before the vendor directory was removed from the repo, `[patch]` section deleted), same 45 s scenario |
| `soak-2026-11-08.*` (`.log.gz`) | Repo build, 20 bots, 30 min, 3600 msgs at 2/s. Details in `results/soak-2026-11-08.md` |

The run with a vendored `wasip3` bumped to wit-bindgen 0.63 (single async runtime) is not in `results/`: it was a 45 s
run, 100/100 messages, same clock and no kicks, and its files were deleted.

| | Short (`short-45s`) | Soak |
|---|---|---|
| Messages received / sent | 100 / 100 | 3600 / 3600 |
| Latency median / p99 / max | 1 / 63 / 72 ms | 0 / 2 / 69 ms |
| CLOCK mean per 10 s window | 50.0 ms (6 windows) | 50.0 ms (182 windows) |
| Max tick interval / > 200 ms | 125.7 ms / 0 | 100.3 ms / 0 |
| Joins / chats logged | 10 / 120 | 20 / 11899 |
| Bots kicked / connected at end | 0 / 10 of 10 | 0 / 20 of 20 |
| Errors, panics, traps | none | none |
| RAM start / end | 128 / 303 MiB | 50 / 306 MiB |
| RAM slope after 5 min warm-up | n/a | +20 MiB per 10 min (not flat, cause not separated, see soak file) |
| CPU mean | 13 % | 15 % |

The ~70 ms latency maxima fall in the 10 s window where the bots join and load chunks.
`results/short-45s-<sha>.txt`-style files from later commits record re-validations of the fixed scripts.

The image used for these runs was built from the fork's merge commit (same tree hash) with the Dockerfile this repo
had before it gained a `HEALTHCHECK` (`nc -z 127.0.0.1 25565`); the current Dockerfile includes it. No effect on the results.

## Files

- `probe/`: the plugin (`src/lib.rs`).
- `scripts/`: `run.sh`, `publish.py`, `analyze.py`, `sample-stats.sh`, `fetch-proxy.sh`, `checkout-pumpkin.sh`.
- `bots/`: Mineflayer load script and lockfile.
- `config/`: `pumpkin.toml` (secret placeholder), `velocity.toml`.
- `Dockerfile`: server build. `docker-compose.yml`: redis, pumpkin, velocity.

Written with AI assistance (Claude Code).

License: MIT OR Apache-2.0.
