use log::*;
use solana_sdk::{hash::Hash, message::VersionedMessage};
use std::sync::Arc;

use solana_sdk::{
    message::v0::Message,
    signature::{Keypair, Signature},
    signer::Signer,
    system_instruction, system_program,
    transaction::VersionedTransaction,
};
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
    let rpc_client = utils::create_rpc_client(&url);

    let auth = Keypair::new();
    let new_account = Keypair::new();
    let ix = system_instruction::create_account(
        &auth.pubkey(),
        &new_account.pubkey(),
        1_000_000_000,
        0,
        &system_program::ID,
    );

    let versioned_msg = Message::try_compile(&auth.pubkey(), &[ix], &[], Hash::default()).unwrap();
    let versioned_tx =
        VersionedTransaction::try_new(VersionedMessage::V0(versioned_msg), &[&auth, &new_account])
            .unwrap();
    let sig = rpc_client
        .send_and_confirm_transaction(&versioned_tx)
        .await
        .unwrap();

    debug!("Transaction sent with signature: {}", sig);

    utils::stop(handle).await;
}
