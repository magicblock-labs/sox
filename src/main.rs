use log::*;
use std::net::SocketAddr;

use errors::ResponderRpcResult;
use jsonrpsee::server::{Server, ServerHandle};
use responder::{create_rpc_module, ResponderConfig};

mod errors;
mod responder;
mod rpc;

#[tokio::main]
async fn main() {
    let config = ResponderConfig::development();
    match start_rpc_server(config).await {
        Ok((url, _handle)) => {
            info!("RPC server started at {}", url);
        }
        Err(e) => eprintln!("Failed to start RPC server: {}", e),
    }
}

const DEFAULT_RPC_URL: &str = "http://localhost:8899";
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
