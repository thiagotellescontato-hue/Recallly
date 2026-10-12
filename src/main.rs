mod probe;
mod timezone;

use recallly_probe::storage::{self, Store, SystemClock};
use rmcp::ServiceExt;
use std::process::ExitCode;
use std::sync::Arc;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let store = (|| -> storage::Result<Store> {
        let path = storage::database_path()?;
        let timeout = storage::claim_timeout()?;
        let mut store = Store::open(&path, Arc::new(SystemClock))?;
        store.recover_abandoned(timeout)?;
        Ok(store)
    })()
    .map_err(|error| error.to_string())?;
    let failure = || "Recallly Probe could not initialize or complete the MCP session.".to_owned();
    let probe = probe::Probe::new(timezone::detected_timezone().map_err(|_| failure())?)
        .map_err(|_| failure())?;
    let service = probe
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|_| failure())?;
    service.waiting().await.map_err(|_| failure())?;
    drop(store);
    Ok(())
}
