use solana_rpc_client_api::response::RpcSimulateTransactionResult;
use solana_sdk::transaction::VersionedTransaction;
use solana_transaction_status::ConfirmedTransactionStatusWithSignature;

pub enum TransactionResult {
    SimulationError(RpcSimulateTransactionResult),
    SignatureStatus(ConfirmedTransactionStatusWithSignature),
}

pub trait SoxMocker: Send + Sync + 'static {
    fn handle_transaction(&self, _tx: VersionedTransaction) -> Option<TransactionResult> {
        // By default we pass the transaction to the proxied validator
        None
    }
}

pub struct NoOpSoxMocker;
impl SoxMocker for NoOpSoxMocker {}
