use std::sync::Arc;

use log::*;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use sox::{mocker::SoxMocker, start_rpc_server, ResponderConfig, ResponderRpcResult, ServerHandle};

fn init_logger() {
    let _ = env_logger::builder().format_timestamp(None).try_init();
}

pub async fn start<M: SoxMocker>(mocker: Arc<M>) -> ResponderRpcResult<(String, ServerHandle)> {
    init_logger();

    let config = ResponderConfig::noproxy();
    match start_rpc_server(mocker, config).await {
        Ok((url, handle)) => {
            info!("RPC server started at {}", url);
            Ok((url, handle))
        }
        Err(e) => {
            error!("Failed to start RPC server: {}", e);
            Err(e)
        }
    }
}

pub async fn stop(handle: ServerHandle) {
    debug!("Stopping RPC server...");
    handle.stop().unwrap();
    handle.stopped().await;
    debug!("RPC server stopped.");
}

pub(crate) async fn create_rpc_client(url: &str) -> RpcClient {
    todo!("create_rpc_client");
}
