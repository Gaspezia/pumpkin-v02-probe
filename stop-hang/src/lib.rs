//! Env STOPHANG_VARIANT: "a" = endless future, "b" = same loop, ends when on_unload sets a flag.
use pumpkin_plugin_api_v0_2::{Context, Plugin, PluginMetadata, Result, register_plugin};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

static STOP: AtomicBool = AtomicBool::new(false);
struct StopHang;

impl Plugin for StopHang {
    fn new() -> Self { StopHang }
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "stop-hang".into(), version: "0.1.0".into(), authors: vec![],
            description: "stop hang repro".into(), dependencies: vec![], permissions: vec!["sys.env".into()],
        }
    }
    async fn on_load(&self, _c: Context) -> Result<()> {
        let coop = std::env::var("STOPHANG_VARIANT").is_ok_and(|v| v == "b");
        wit_bindgen::spawn_local(async move {
            while !(coop && STOP.load(Relaxed)) {
                wasip3::clocks::monotonic_clock::wait_for(200_000_000).await; // 200 ms
            }
            tracing::info!("stop-hang: loop ended");
        });
        tracing::info!("stop-hang: on_load done, coop={coop}");
        Ok(())
    }
    async fn on_unload(&self, _c: Context) -> Result<()> {
        tracing::info!("stop-hang: on_unload called");
        STOP.store(true, Relaxed);
        Ok(())
    }
}
register_plugin!(StopHang);
