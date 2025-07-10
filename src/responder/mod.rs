use crate::mocker::TransactionResult;
use crate::rpc::params::{
    GetAccountInfoParams, GetSignatureStatusesParams, IsBlockhashValidParams,
};
use solana_account_decoder::UiAccount;
use log::*;
use response::response_with_context;
use solana_rpc_client_api::response::Response;
use solana_sdk::transaction::VersionedTransaction;
use solana_transaction_status::TransactionStatus;
use solana_transaction_status::UiTransactionEncoding;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;

use crate::errors::server_error;
use crate::errors::ServerErrorCode;
use crate::mocker::SoxMocker;
use crate::rpc::mockables::register_mockable_methods;
use crate::rpc::params::RawParams;
use crate::rpc::params::SendTransactionParams;
use crate::rpc::transaction::decode_and_deserialize;
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
    mocked_tx_results: Mutex<HashMap<Signature, TransactionResult>>,
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
            mocked_tx_results: HashMap::new().into(),
            tx_results: HashMap::new(),
        })
    }

    pub async fn handle_send_transaction(
        &self,
        params: jsonrpsee::types::Params<'static>,
    ) -> Result<String, ErrorObjectOwned> {
        // TODO: proxy through if not mocked
        let send_tx_params: SendTransactionParams = params.parse().unwrap();
        let encoded_tx = send_tx_params.0;
        let config = send_tx_params.1.unwrap_or_default();

        debug!("Received transaction: {}, {:?}", encoded_tx, config);
        let tx_encoding = config.encoding.unwrap_or(UiTransactionEncoding::Base58);
        let binary_encoding = tx_encoding.into_binary_encoding().ok_or_else(|| {
            server_error(
                format!("unsupported encoding: {tx_encoding}. Supported encodings: base58, base64"),
                ServerErrorCode::RpcClientError,
            )
        })?;
        let (_, decoded_tx) =
            decode_and_deserialize::<VersionedTransaction>(encoded_tx, binary_encoding).map_err(
                |err| {
                    server_error(
                        format!("Failed to decode transaction: {err}"),
                        ServerErrorCode::RpcClientError,
                    )
                },
            )?;
        debug!("Decoded transaction: {:?}", decoded_tx);

        let signature = *decoded_tx.signatures.first().ok_or_else(|| {
            server_error(
                "Transaction has no signatures".to_string(),
                ServerErrorCode::RpcClientError,
            )
        })?;
        if let Some(result) = self.mocker.handle_transaction(decoded_tx) {
            debug!("Mocked transaction result: {:?}", result);
            let sig_str = signature.to_string();
            self.mocked_tx_results
                .lock()
                .expect("mocked_tx_results mutex poisoned")
                .insert(signature, result);
            Ok(sig_str)
        } else {
            todo!("Send transaction to remote cluster if not mocked");
        }
    }

    pub async fn handle_get_signature_statuses(
        &self,
        params: jsonrpsee::types::Params<'static>,
    ) -> Result<Response<Vec<Option<TransactionStatus>>>, ErrorObjectOwned> {
        let get_signature_statuses_params: GetSignatureStatusesParams = params.parse().unwrap();
        debug!(
            "handle_get_signature_statuses: {:?}",
            get_signature_statuses_params
        );
        // TODO: try to get missing ones from remote cluster
        let signatures = get_signature_statuses_params.0;
        let mocked_tx_results = self
            .mocked_tx_results
            .lock()
            .expect("mocked_tx_results mutex poisoned");
        let statuses: Vec<Option<TransactionStatus>> = signatures
            .into_iter()
            .map(|sig| {
                let sig = Signature::from_str(&sig).expect("Invalid signature format");
                mocked_tx_results
                    .get(&sig)
                    .and_then(|status| Option::<TransactionStatus>::from(status.clone()))
            })
            .collect();
        debug!("Returning signature statuses: {:?}", statuses);
        Ok(response_with_context(statuses))
    }

    pub async fn handle_is_blockhash_valid(
        &self,
        params: jsonrpsee::types::Params<'static>,
    ) -> Result<Response<bool>, ErrorObjectOwned> {
        let is_blockhash_valid_params: IsBlockhashValidParams = params.parse().unwrap();
        let blockhash = is_blockhash_valid_params.0;
        if let Some(is_valid) = self.mocker.is_blockhash_valid(&blockhash) {
            debug!("Mocked isBlockhashValid result: {:?}", is_valid);
            Ok(response_with_context(is_valid))
        } else {
            self.handle_request("isBlockhashValid", params, None).await
        }
    }

    pub async fn handle_get_account_info(
        &self,
        params: jsonrpsee::types::Params<'static>,
    ) -> Result<Response<Option<UiAccount>>, ErrorObjectOwned> {
        let get_account_info_params: GetAccountInfoParams = params.parse().unwrap();
        let pubkey = get_account_info_params.0;
        let config = get_account_info_params.1;

        if let Some(account_info) = self.mocker.get_account_info(&pubkey, config) {
            debug!("Mocked getAccountInfo result: {:?}", account_info);
            Ok(response_with_context(account_info))
        } else {
            self.handle_request(
                "getAccountInfo",
                params,
                Some(response_with_context(None)),
            )
            .await
        }
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
