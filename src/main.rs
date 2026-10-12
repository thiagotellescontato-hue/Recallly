mod probe;
mod timezone;

use rmcp::ServiceExt;
use std::process::ExitCode;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => {
            eprintln!("Recallly Probe could not initialize or complete the MCP session.");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), ()> {
    let probe = probe::Probe::new(timezone::detected_timezone()?).map_err(|_| ())?;
    let service = probe.serve(rmcp::transport::stdio()).await.map_err(|_| ())?;
    service.waiting().await.map_err(|_| ())?;
    Ok(())
}
