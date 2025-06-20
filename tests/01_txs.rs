use log::*;
use solana_sdk::{hash::Hash, message::VersionedMessage};
use solana_transaction_status::{TransactionConfirmationStatus, TransactionStatus};
use std::sync::Arc;

use solana_sdk::{
    message::v0::Message,
    signature::{Keypair, Signature},
    signer::Signer,
    system_instruction, system_program,
    transaction::VersionedTransaction,
};
use sox::mocker::{SoxMocker, TransactionResult};

mod utils;

fn successful_transaction_status() -> Option<TransactionStatus> {
    Some(TransactionStatus {
        slot: 0,
        confirmations: None,
        status: Ok(()),
        err: None,
        confirmation_status: Some(TransactionConfirmationStatus::Finalized),
    })
}

#[tokio::test]
async fn test_all_txs_success() {
    struct AllTxsSuccessMocker;
    impl SoxMocker for AllTxsSuccessMocker {
        fn handle_transaction(&self, tx: VersionedTransaction) -> Option<TransactionResult> {
            debug!("Mocker received transaction: {:?}", tx);
            Some(TransactionResult::signature_status_success(
                Signature::new_unique(),
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

    let status = rpc_client
        .get_signature_status(&sig)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status, Ok(()));

    let statuses = rpc_client
        .get_signature_statuses(&[sig])
        .await
        .unwrap()
        .value;
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0], successful_transaction_status());

    utils::stop(handle).await;
}
