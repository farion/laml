//! Host for the shared Rust/WASM editor. Deployment policies are opt-in.
mod config;
mod http;
mod limits;
mod process;
mod scoped;
mod stage;
mod worker;
mod workspace;
use config::Config;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
struct ClientProcess {
    process: Arc<tokio::sync::Mutex<process::Process>>,
}
struct HttpState {
    config: Config,
    origin: String,
    assets: cap_std::fs::Dir,
    processes: Mutex<BTreeMap<u64, ClientProcess>>,
    workers: Option<Arc<tokio::sync::Semaphore>>,
}
impl HttpState {
    fn process(&self, client: u64) -> Result<Arc<tokio::sync::Mutex<process::Process>>, String> {
        let mut processes = self
            .processes
            .lock()
            .map_err(|_| "Host state unavailable")?;
        // Silence is not page closure: background browsers suspend polling.
        // Keep the worker and its recovery leases until explicit page departure.
        let entry = processes.entry(client).or_insert_with(|| ClientProcess {
            process: Arc::new(tokio::sync::Mutex::new(process::Process::with_limits(
                self.config.workspace.clone(),
                self.config.state.join("recovery"),
                self.config.limits,
            ))),
        });
        Ok(entry.process.clone())
    }
}
pub fn run(args: &[std::ffi::OsString]) -> Result<(), String> {
    if args.len() == 1 && args[0] == "--help" {
        println!(
            "laml-ui serve --workspace DIR --assets DIR --state-dir DIR [--bind 127.0.0.1:7878] [--origin URL] [--limits] [--parallelism N] [--http-checks] [--require-https]"
        );
        return Ok(());
    }
    let config = Config::parse(args)?;
    let assets = cap_std::fs::Dir::open_ambient_dir(&config.assets, cap_std::ambient_authority())
        .map_err(|_| "Web assets unavailable")?;
    for file in ["laml_ui.js", "laml_ui_bg.wasm"] {
        if !assets.metadata(file).is_ok_and(|m| m.is_file()) {
            return Err("Build web assets first with scripts/build-editor-web.sh".into());
        }
    }
    config::private_dir(&config.state.join("recovery"))?;
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|_| "Could not start host runtime")?
        .block_on(async move {
            let listener = tokio::net::TcpListener::bind(config.bind)
                .await
                .map_err(|_| "Could not bind host address")?;
            let origin = config.public_origin(
                listener
                    .local_addr()
                    .map_err(|_| "Host address unavailable")?,
            )?;
            eprintln!("LAML host: {origin}");
            let workers = config
                .parallelism
                .map(|n| Arc::new(tokio::sync::Semaphore::new(n)));
            let state = Arc::new(HttpState {
                config,
                origin,
                assets,
                processes: Mutex::new(BTreeMap::new()),
                workers,
            });
            axum::serve(listener, http::router(state))
                .await
                .map_err(|_| "Host listener stopped".into())
        })
}
pub fn worker_main(
    workspace: std::path::PathBuf,
    recovery: std::path::PathBuf,
    limits: bool,
) -> Result<(), String> {
    worker::main(workspace, recovery, limits)
}
#[cfg(test)]
mod tests;
