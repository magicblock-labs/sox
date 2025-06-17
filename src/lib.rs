use std::{net::SocketAddr, sync::Arc};

use jsonrpsee::server::Server;
use mocker::SoxMocker;
use responder::create_rpc_module;

pub use errors::ResponderRpcResult;
pub use jsonrpsee::server::ServerHandle;
pub use responder::ResponderConfig;

mod errors;
pub mod mocker;
mod responder;
mod rpc;

const DEFAULT_RPC_URL: &str = "127.0.0.1:8899";
pub async fn start_rpc_server<M: SoxMocker>(
    mocker: Arc<M>,
    config: ResponderConfig,
) -> ResponderRpcResult<(String, ServerHandle)> {
    let url = DEFAULT_RPC_URL;

    let server = Server::builder()
        .http_only()
        .build(url.parse::<SocketAddr>().unwrap())
        .await?;

    let rpc_module = create_rpc_module(mocker, config)?;
    let handle = server.start(rpc_module);
    Ok((url.to_string(), handle))
}
