mod config;
mod embedded_assets;
mod gsi;
mod invoker;
mod overlay;

use std::sync::Arc;

use anyhow::Context;
use dota::ServerBuilder;
use parking_lot::RwLock;

use crate::config::AppConfig;
use crate::invoker::CooldownState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::load().context("failed to load dota_2_gsi_invoker_config.json")?;
    let addr = format!("127.0.0.1:{}", config.gsi_port);
    eprintln!(
        "dota_2_gsi_invoker listening on http://{addr}/ (debug_gsi={}, show_footer_row={})",
        config.debug_gsi, config.show_footer_row,
    );
    eprintln!("waiting for Invoker pick");
    let state = Arc::new(RwLock::new(CooldownState::new()));

    start_gsi_server(addr.clone(), Arc::clone(&state), config.debug_gsi)
        .context("failed to start Dota 2 GSI server")?;
    overlay::run(state, config).map_err(|err| anyhow::anyhow!("overlay failed: {err}"))?;

    Ok(())
}

fn start_gsi_server(
    addr: String,
    state: Arc<RwLock<CooldownState>>,
    debug_gsi: bool,
) -> anyhow::Result<()> {
    let server = ServerBuilder::new(&addr)
        .register(move |payload: bytes::Bytes| {
            let state = Arc::clone(&state);
            async move {
                if debug_gsi {
                    match serde_json::from_slice::<serde_json::Value>(&payload) {
                        Ok(value) => println!("{}", serde_json::to_string_pretty(&value)?),
                        Err(_) => println!("{}", String::from_utf8_lossy(&payload)),
                    }
                }

                match gsi::parse_invoker_cooldowns(&payload) {
                    Ok(update) => {
                        state.write().apply(update);
                    }
                    Err(err) => {
                        eprintln!("ignored malformed GSI payload: {err:#}");
                    }
                }

                Ok::<_, anyhow::Error>(())
            }
        })
        .start()?;

    tokio::spawn(async move {
        server.run_forever().await;
    });

    Ok(())
}
