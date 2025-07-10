use std::{net::SocketAddr, sync::Arc};

pub use errors::ResponderRpcResult;
use jsonrpsee::server::Server;
pub use jsonrpsee::server::ServerHandle;
use mocker::SoxMocker;
use responder::create_rpc_module;
pub use responder::ResponderConfig;

mod errors;
pub mod mocker;
mod responder;
mod rpc;

const DEFAULT_RPC_PORT: u16 = 8899;
pub async fn start_rpc_server<M: SoxMocker>(
    mocker: Arc<M>,
    config: ResponderConfig,
    port: Option<u16>,
) -> ResponderRpcResult<(String, ServerHandle)> {
    let port = port.unwrap_or(DEFAULT_RPC_PORT);
    let url = format!("127.0.0.1:{port}");
    let server = Server::builder()
        .http_only()
        .build(url.parse::<SocketAddr>().unwrap())
        .await?;

    let rpc_module = create_rpc_module(mocker, config)?;
    let handle = server.start(rpc_module);
    Ok((url.to_string(), handle))
}
