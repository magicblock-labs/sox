use solana_account_decoder::UiAccount;
use solana_rpc_client_api::config::RpcAccountInfoConfig;
use solana_rpc_client_api::response::RpcSimulateTransactionResult;
use solana_sdk::{
    signature::Signature,
    transaction::{TransactionError, VersionedTransaction},
};
use solana_transaction_status::{
    ConfirmedTransactionStatusWithSignature, TransactionConfirmationStatus, TransactionStatus,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionResult {
    /// The RPC does not handle the transaction (dropping it).
    Drop,
    /// A simulation error occurred.
    SimulationError(RpcSimulateTransactionResult),
    /// A transaction was handled with a valid result.
    SignatureStatus(ConfirmedTransactionStatusWithSignature),
}

impl TransactionResult {
    pub fn signature_status_success(signature: Signature) -> Self {
        TransactionResult::SignatureStatus(ConfirmedTransactionStatusWithSignature {
            signature,
            slot: 0,
            err: None,
            memo: None,
            block_time: None,
        })
    }

    pub fn signature_status_error(signature: Signature, err: TransactionError) -> Self {
        TransactionResult::SignatureStatus(ConfirmedTransactionStatusWithSignature {
            signature,
            slot: 0,
            err: Some(err),
            memo: None,
            block_time: None,
        })
    }
}

impl From<TransactionResult> for Option<TransactionStatus> {
    fn from(value: TransactionResult) -> Option<TransactionStatus> {
        use TransactionResult::*;
        match value {
            Drop | SimulationError(_) => None,
            SignatureStatus(status) => Some(TransactionStatus {
                slot: status.slot,
                confirmations: None,
                status: if status.err.is_some() {
                    Err(status.err.clone().unwrap())
                } else {
                    Ok(())
                },
                err: status.err,
                confirmation_status: Some(TransactionConfirmationStatus::Finalized),
            }),
        }
    }
}

pub trait SoxMocker: Send + Sync + 'static {
    fn handle_transaction(&self, _tx: VersionedTransaction) -> Option<TransactionResult> {
        // By default we pass the transaction to the proxied validator
        None
    }

    fn is_blockhash_valid(&self, _blockhash: &str) -> Option<bool> {
        // By default we pass the request to the proxied validator
        None
    }

    fn get_account_info(
        &self,
        _pubkey: &str,
        _config: Option<RpcAccountInfoConfig>,
    ) -> Option<Option<UiAccount>> {
        // By default we pass the request to the proxied validator
        None
    }
}

pub struct NoOpSoxMocker;
impl SoxMocker for NoOpSoxMocker {}
