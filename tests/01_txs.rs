use log::*;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{hash::Hash, message::VersionedMessage, transaction::TransactionError};
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

fn create_account_tx() -> VersionedTransaction {
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

    VersionedTransaction::try_new(VersionedMessage::V0(versioned_msg), &[&auth, &new_account])
        .unwrap()
}

async fn sig_status(rpc_client: &RpcClient, sig: Signature) -> Result<(), TransactionError> {
    rpc_client
        .get_signature_status(&sig)
        .await
        .unwrap()
        .unwrap()
}

async fn sig_statuses(
    rpc_client: &RpcClient,
    sigs: &[Signature],
) -> Vec<Option<TransactionStatus>> {
    rpc_client.get_signature_statuses(sigs).await.unwrap().value
}

#[tokio::test]
async fn test_one_tx_success() {
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

    let tx = create_account_tx();
    let sig = rpc_client.send_and_confirm_transaction(&tx).await.unwrap();
    let (status, statuses) = (
        sig_status(&rpc_client, sig).await,
        sig_statuses(&rpc_client, &[sig]).await,
    );

    assert_eq!(status, Ok(()));
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0], successful_transaction_status());

    utils::stop(handle).await;
}

#[tokio::test]
async fn test_two_tx_success() {
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

    let tx1 = create_account_tx();
    let sig1 = rpc_client.send_and_confirm_transaction(&tx1).await.unwrap();
    let status1 = sig_status(&rpc_client, sig1).await;

    let tx2 = create_account_tx();
    let sig2 = rpc_client.send_and_confirm_transaction(&tx2).await.unwrap();
    let status2 = sig_status(&rpc_client, sig2).await;

    assert_eq!(status1, Ok(()));
    assert_eq!(status2, Ok(()));

    let statuses = sig_statuses(&rpc_client, &[sig1, sig2]).await;
    assert_eq!(statuses.len(), 2);
    assert_eq!(statuses[0], successful_transaction_status());
    assert_eq!(statuses[1], successful_transaction_status());

    utils::stop(handle).await;
}
