use solana_rpc_client_api::response::RpcSimulateTransactionResult;
use solana_sdk::{signature::Signature, transaction::VersionedTransaction};
use solana_transaction_status::{
    ConfirmedTransactionStatusWithSignature, TransactionConfirmationStatus, TransactionStatus,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionResult {
    SimulationError(RpcSimulateTransactionResult),
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
}

impl From<TransactionResult> for Option<TransactionStatus> {
    fn from(value: TransactionResult) -> Option<TransactionStatus> {
        use TransactionResult::*;
        match value {
            SimulationError(_) => None,
            SignatureStatus(status) => Some(TransactionStatus {
                slot: status.slot,
                confirmations: None,
                status: Ok(()),
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
}

pub struct NoOpSoxMocker;
impl SoxMocker for NoOpSoxMocker {}
