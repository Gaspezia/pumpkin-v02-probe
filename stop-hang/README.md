# stop-hang

Repro for a hang on shutdown: `stop` and SIGTERM never return while a v0.2 plugin keeps an endless `wit_bindgen::spawn_local` future.

The plugin (`src/lib.rs`, about 15 lines of logic) starts one future in `on_load` that loops on `wasip3::clocks::monotonic_clock::wait_for(200 ms)`.
`STOPHANG_VARIANT` selects the variant:

- `a` (default): the loop never ends.
- `b`: the same loop ends when `on_unload` sets a flag.

No network, no Redis, no players.

Same server and SDK as the parent probe: upstream master `eda69440e` merged with PR #3962 head `0740d3f6`, `--features plugin-v0_2`.

```bash
# from the repo root
scripts/checkout-pumpkin.sh                          # ./pumpkin-src = master + PR head
docker compose build pumpkin                         # image pumpkin-v02-probe:local, about 30 min
cd stop-hang
cargo build --release --target wasm32-wasip2         # target/wasm32-wasip2/release/stop_hang.wasm
./run.sh none stop                                   # no plugin: exits in under 1 s
./run.sh a stop                                      # variant a, console stop: keeps running
./run.sh a term                                      # variant a, SIGTERM through docker stop -t 30: killed after 30 s, exit 137
./run.sh b stop                                      # variant b: exits in under 1 s
```

`run.sh <none|a|b> <stop|term>` prints the seconds to exit, the exit code and the last log lines. No host port is published.
Set `PUMPKIN_IMAGE` to use another server image. Results from the run that goes with the report are in the GitHub issue.
