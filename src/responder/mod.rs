use log::*;
use std::collections::HashMap;
use std::sync::Arc;

use crate::errors::server_error;
use crate::errors::ServerErrorCode;
use crate::mocker::SoxMocker;
use crate::rpc::params::RawParams;
use crate::rpc::params::SendTransactionParams;
use crate::rpc::passthrough::register_mockable_methods;
use cluster::RpcCluster;
use jsonrpsee::{
    core::{client::ClientT, ClientError},
    http_client::{HttpClient, HttpClientBuilder},
    types::ErrorObjectOwned,
    RpcModule,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use solana_sdk::signature::Signature;
use solana_transaction_status::ConfirmedTransactionStatusWithSignature;
mod cluster;
pub mod response;

use crate::{errors::ResponderRpcResult, rpc::passthrough::register_passthrough_methods};

#[derive(Debug, Clone)]
pub struct ResponderConfig {
    /// The cluster to proxy requests to.
    /// If `None`, the responder will not proxy requests.
    pub remote_cluster: Option<RpcCluster>,
}

impl ResponderConfig {
    pub fn noproxy() -> Self {
        Self {
            remote_cluster: None,
        }
    }

    pub fn development() -> Self {
        Self {
            remote_cluster: Some(RpcCluster::Development),
        }
    }
    pub fn devnet() -> Self {
        Self {
            remote_cluster: Some(RpcCluster::Devnet),
        }
    }
}

#[allow(unused)]
pub struct ResponderRpc<M: SoxMocker> {
    pub(super) rpc_remote_client: Option<HttpClient>,
    pub(super) mocker: Arc<M>,
    tx_results: HashMap<Signature, ConfirmedTransactionStatusWithSignature>,
}

pub fn create_rpc_module<M: SoxMocker>(
    mocker: Arc<M>,
    config: ResponderConfig,
) -> ResponderRpcResult<RpcModule<ResponderRpc<M>>> {
    let responder = ResponderRpc::try_new(mocker, config)?;
    let mut module = RpcModule::new(responder);
    register_mockable_methods(&mut module)?;
    register_passthrough_methods(&mut module)?;

    Ok(module)
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct NoProxy {
    value: String,
}

impl<M: SoxMocker> ResponderRpc<M> {
    fn try_new(mocker: Arc<M>, config: ResponderConfig) -> ResponderRpcResult<Self> {
        let rpc_remote_client = config
            .remote_cluster
            .as_ref()
            .map(|x| HttpClientBuilder::default().build(x.url()))
            .transpose()?;

        Ok(Self {
            rpc_remote_client,
            mocker,
            tx_results: HashMap::new(),
        })
    }

    pub async fn handle_send_transaction(
        &self,
        params: jsonrpsee::types::Params<'static>,
    ) -> Result<String, ErrorObjectOwned> {
        // TODO: let mocker handle this
        // TODO: proxy through if not mocked
        let send_tx_params: SendTransactionParams = params.parse().unwrap();
        let encoded_tx = send_tx_params.0;
        debug!("Received transaction: {}", encoded_tx);

        let signature = Signature::new_unique();
        Ok(signature.to_string())
    }

    pub async fn handle_request<R: DeserializeOwned>(
        &self,
        method: &str,
        params: jsonrpsee::types::Params<'static>,
        default_value: Option<R>,
    ) -> Result<R, ErrorObjectOwned> {
        let Some(rpc_remote_client) = self.rpc_remote_client.as_ref() else {
            if let Some(default_value) = default_value {
                return Ok(default_value);
            } else {
                return Err(server_error(
                    format!(
                        "Responder is not configured to proxy requests and no default value is known for {}",
                        method
                    ),
                    ServerErrorCode::RpcClientError,
                ));
            }
        };

        match rpc_remote_client
            .request::<R, RawParams>(method, RawParams(params))
            .await
        {
            Ok(res) => Ok(res),
            Err(err) => match err {
                // Pass RPC JSON errors through directly
                ClientError::Call(err) => Err(err),
                _ => Err(server_error(
                    format!("Failed to forward to proxied RPC: {err:?}"),
                    ServerErrorCode::RpcClientError,
                )),
            },
        }
    }
}
