use log::*;
use std::net::SocketAddr;

use errors::ResponderRpcResult;
use jsonrpsee::server::{Server, ServerHandle};
use responder::{create_rpc_module, ResponderConfig};

mod errors;
mod responder;
mod rpc;

fn init_logger() {
    let _ = env_logger::builder().format_timestamp(None).try_init();
}

#[tokio::main]
async fn main() {
    init_logger();

    let config = ResponderConfig::noproxy();
    match start_rpc_server(config).await {
        Ok((url, handle)) => {
            info!("RPC server started at {}", url);
            handle.stopped().await;
        }
        Err(e) => error!("Failed to start RPC server: {}", e),
    }
}

const DEFAULT_RPC_URL: &str = "127.0.0.1:8899";
pub async fn start_rpc_server(
    config: ResponderConfig,
) -> ResponderRpcResult<(String, ServerHandle)> {
    let url = DEFAULT_RPC_URL;

    let server = Server::builder()
        .http_only()
        .build(url.parse::<SocketAddr>().unwrap())
        .await?;

    let rpc_module = create_rpc_module(config)?;
    let handle = server.start(rpc_module);
    Ok((url.to_string(), handle))
}
