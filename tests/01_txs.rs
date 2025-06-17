use log::*;
use std::sync::Arc;

use solana_sdk::{signature::Signature, transaction::VersionedTransaction};
use solana_transaction_status::ConfirmedTransactionStatusWithSignature;
use sox::mocker::{SoxMocker, TransactionResult};

mod utils;

#[tokio::test]
async fn test_all_txs_success() {
    struct AllTxsSuccessMocker;
    impl SoxMocker for AllTxsSuccessMocker {
        fn handle_transaction(&self, _tx: VersionedTransaction) -> Option<TransactionResult> {
            debug!("Mocker received transaction: {:#?}", _tx);
            let signature = Signature::new_unique();
            Some(TransactionResult::SignatureStatus(
                ConfirmedTransactionStatusWithSignature {
                    signature,
                    slot: 0,
                    err: None,
                    memo: None,
                    block_time: None,
                },
            ))
        }
    }

    let mocker = Arc::new(AllTxsSuccessMocker);
    let (url, handle) = utils::start(mocker).await.unwrap();

    let rpc_client = utils::create_rpc_client(url).await.unwrap();

    utils::stop(handle).await;
}
