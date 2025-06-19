use solana_rpc_client_api::response::RpcSimulateTransactionResult;
use solana_sdk::transaction::VersionedTransaction;
use solana_transaction_status::{
    ConfirmedTransactionStatusWithSignature, TransactionConfirmationStatus, TransactionStatus,
};

#[derive(Debug, Clone)]
pub enum TransactionResult {
    SimulationError(RpcSimulateTransactionResult),
    SignatureStatus(ConfirmedTransactionStatusWithSignature),
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
