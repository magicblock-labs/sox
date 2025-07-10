use std::sync::Arc;

use log::*;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use sox::{
    mocker::SoxMocker, start_rpc_server, ResponderConfig, ResponderRpcResult,
    ServerHandle,
};

fn init_logger() {
    let _ = env_logger::builder()
        .format_source_path(true)
        .format_timestamp(None)
        .try_init();
}

pub async fn start<M: SoxMocker>(
    mocker: Arc<M>,
) -> ResponderRpcResult<(String, ServerHandle)> {
    start_with_config(mocker, ResponderConfig::noproxy()).await
}

pub async fn start_with_config<M: SoxMocker>(
    mocker: Arc<M>,
    config: ResponderConfig,
) -> ResponderRpcResult<(String, ServerHandle)> {
    init_logger();

    let port = rand::random::<u16>() % 10000 + 20000; // Port range 20000-29999
    match start_rpc_server(mocker, config, Some(port)).await {
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

pub(crate) fn create_rpc_client(url: &str) -> RpcClient {
    RpcClient::new_with_commitment(
        format!("http://{url}"),
        CommitmentConfig::processed(),
    )
}
