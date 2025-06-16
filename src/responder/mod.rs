use crate::errors::server_error;
use crate::errors::ServerErrorCode;
use crate::rpc::params::RawParams;
use cluster::RpcCluster;
use jsonrpsee::{
    core::{client::ClientT, ClientError},
    http_client::{HttpClient, HttpClientBuilder},
    types::ErrorObjectOwned,
    RpcModule,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
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
