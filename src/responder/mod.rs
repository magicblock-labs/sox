use crate::errors::server_error;
use crate::errors::ServerErrorCode;
use cluster::RpcCluster;
use jsonrpsee::{
    core::{client::ClientT, ClientError},
    http_client::{HttpClient, HttpClientBuilder},
    types::ErrorObjectOwned,
    RpcModule,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
mod cluster;

use crate::{
    errors::ResponderRpcResult,
    rpc::{params::RawParams, passthrough::register_passthrough_methods},
};

#[derive(Default, Debug, Clone)]
pub struct ResponderConfig {
    /// The cluster to proxy requests to.
    /// If `None`, the responder will not proxy requests.
    pub remote_cluster: Option<RpcCluster>,
}

impl ResponderConfig {
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

pub struct ResponderRpc {
    pub(super) rpc_remote_client: Option<HttpClient>,
}

pub fn create_rpc_module(config: ResponderConfig) -> ResponderRpcResult<RpcModule<ResponderRpc>> {
    let director = ResponderRpc::try_new(config)?;
    let mut module = RpcModule::new(director);
    register_passthrough_methods(&mut module)?;

    Ok(module)
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct NoProxy {
    value: String,
}

impl ResponderRpc {
    fn try_new(config: ResponderConfig) -> ResponderRpcResult<Self> {
        let rpc_remote_client = config
            .remote_cluster
            .as_ref()
            .map(|x| HttpClientBuilder::default().build(x.url()))
            .transpose()?;

        Ok(Self { rpc_remote_client })
    }

    pub async fn handle_request<R: DeserializeOwned>(
        &self,
        method: &str,
        params: jsonrpsee::types::Params<'static>,
    ) -> Result<R, ErrorObjectOwned> {
        let Some(rpc_remote_client) = self.rpc_remote_client.as_ref() else {
            let no_proxy = NoProxy {
                value: "Responder is not configured to proxy requests".to_string(),
            };

            // Serialize NoProxy to JSON and then deserialize as R
            let json_value = serde_json::to_value(no_proxy).map_err(|e| {
                server_error(
                    format!("Failed to serialize no-proxy response: {e:?}"),
                    ServerErrorCode::RpcClientError,
                )
            })?;

            let result = serde_json::from_value::<R>(json_value).map_err(|e| {
                server_error(
                    format!("Failed to deserialize no-proxy response as expected type: {e:?}"),
                    ServerErrorCode::RpcClientError,
                )
            })?;

            return Ok(result);
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
                    format!("Failed to forward to on-chain RPC: {err:?}"),
                    ServerErrorCode::RpcClientError,
                )),
            },
        }
    }
}
