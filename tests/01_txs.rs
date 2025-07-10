use std::sync::{Arc, Mutex};

use log::*;
use solana_rpc_client::{
    nonblocking::rpc_client::RpcClient, rpc_client::SerializableTransaction,
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    hash::Hash,
    message::{v0::Message, VersionedMessage},
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    system_instruction, system_program,
    transaction::{TransactionError, VersionedTransaction},
};
use solana_transaction_status::{
    TransactionConfirmationStatus, TransactionStatus,
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

    let versioned_msg =
        Message::try_compile(&auth.pubkey(), &[ix], &[], Hash::default())
            .unwrap();

    VersionedTransaction::try_new(
        VersionedMessage::V0(versioned_msg),
        &[&auth, &new_account],
    )
    .unwrap()
}

#[allow(dead_code)]
fn transfer_tx() -> (Pubkey, VersionedTransaction) {
    let auth = Keypair::new();
    let recipient = Keypair::new();
    let ix = system_instruction::transfer(
        &auth.pubkey(),
        &recipient.pubkey(),
        1_000_000_000,
    );

    let versioned_msg =
        Message::try_compile(&auth.pubkey(), &[ix], &[], Hash::default())
            .unwrap();
    (
        auth.pubkey(),
        VersionedTransaction::try_new(
            VersionedMessage::V0(versioned_msg),
            &[&auth],
        )
        .unwrap(),
    )
}

async fn sig_status(
    rpc_client: &RpcClient,
    sig: Signature,
) -> Result<(), TransactionError> {
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

// -----------------
// All Success Mocks
// -----------------
#[tokio::test]
async fn test_one_tx_success() {
    struct AllTxsSuccessMocker;
    impl SoxMocker for AllTxsSuccessMocker {
        fn handle_transaction(
            &self,
            tx: VersionedTransaction,
        ) -> Option<TransactionResult> {
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
        fn handle_transaction(
            &self,
            tx: VersionedTransaction,
        ) -> Option<TransactionResult> {
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

// -----------------
// All Failure Mocks
// -----------------
#[tokio::test]
async fn test_one_tx_failure() {
    struct AllTxsFailureMocker;
    impl SoxMocker for AllTxsFailureMocker {
        fn handle_transaction(
            &self,
            tx: VersionedTransaction,
        ) -> Option<TransactionResult> {
            debug!("Mocker received transaction: {:?}", tx);
            Some(TransactionResult::signature_status_error(
                Signature::new_unique(),
                TransactionError::AccountInUse,
            ))
        }
    }

    let mocker = Arc::new(AllTxsFailureMocker);
    let (url, handle) = utils::start(mocker).await.unwrap();
    let rpc_client = utils::create_rpc_client(&url);

    let tx = create_account_tx();
    let res = rpc_client.send_and_confirm_transaction(&tx).await;
    assert!(res.is_err());

    let sig = tx.get_signature();
    let (status, statuses) = (
        sig_status(&rpc_client, *sig).await,
        sig_statuses(&rpc_client, &[*sig]).await,
    );

    assert!(status.is_err());
    assert_eq!(statuses.len(), 1);
    assert!(statuses[0].as_ref().unwrap().err.is_some());

    utils::stop(handle).await;
}

#[tokio::test]
async fn test_two_tx_failure() {
    struct AllTxsFailureMocker;
    impl SoxMocker for AllTxsFailureMocker {
        fn handle_transaction(
            &self,
            tx: VersionedTransaction,
        ) -> Option<TransactionResult> {
            debug!("Mocker received transaction: {:?}", tx);
            Some(TransactionResult::signature_status_error(
                Signature::new_unique(),
                TransactionError::AccountInUse,
            ))
        }
    }

    let mocker = Arc::new(AllTxsFailureMocker);
    let (url, handle) = utils::start(mocker).await.unwrap();
    let rpc_client = utils::create_rpc_client(&url);

    let tx1 = create_account_tx();
    let res1 = rpc_client.send_and_confirm_transaction(&tx1).await;
    assert!(res1.is_err());

    let tx2 = create_account_tx();
    let res2 = rpc_client.send_and_confirm_transaction(&tx2).await;
    assert!(res2.is_err());

    let sig1 = tx1.get_signature();
    let sig2 = tx2.get_signature();

    let statuses = sig_statuses(&rpc_client, &[*sig1, *sig2]).await;
    assert_eq!(statuses.len(), 2);
    assert!(statuses[0].as_ref().unwrap().err.is_some());
    assert!(statuses[1].as_ref().unwrap().err.is_some());

    utils::stop(handle).await;
}

// -----------------
// Mixed Success/Failure/Drop Mocks
// -----------------
// #[tokio::test]
#[allow(dead_code)]
async fn test_failing_for_specific_payer() {
    struct FailForSpecificPayerMock {
        payer: Pubkey,
    }

    impl SoxMocker for FailForSpecificPayerMock {
        fn handle_transaction(
            &self,
            tx: VersionedTransaction,
        ) -> Option<TransactionResult> {
            debug!("Mocker received transaction: {:?}", tx);
            if tx.message.static_account_keys().contains(&self.payer) {
                Some(TransactionResult::signature_status_error(
                    Signature::new_unique(),
                    TransactionError::AccountInUse,
                ))
            } else {
                Some(TransactionResult::signature_status_success(
                    Signature::new_unique(),
                ))
            }
        }
    }

    let (payer, tx_to_fail) = transfer_tx();
    let mocker = Arc::new(FailForSpecificPayerMock { payer });

    let (url, handle) = utils::start(mocker).await.unwrap();
    let rpc_client = utils::create_rpc_client(&url);

    // TODO: @@@ fix this, we would expect a signature and only when we get
    // a status would we get the error
    let sig = rpc_client
        .send_and_confirm_transaction(&tx_to_fail)
        .await
        .unwrap();
    let status = sig_status(&rpc_client, sig).await;
    assert!(status.is_err());

    let statuses = sig_statuses(&rpc_client, &[sig]).await;
    assert_eq!(statuses.len(), 1);
    assert!(statuses[0].as_ref().unwrap().err.is_some());

    utils::stop(handle).await;
}

#[tokio::test]
async fn test_two_tx_first_one_dropped_second_fails_third_succeeds() {
    struct DropFailSucceedMocker {
        count: Mutex<u8>,
    }
    impl SoxMocker for DropFailSucceedMocker {
        fn handle_transaction(
            &self,
            tx: VersionedTransaction,
        ) -> Option<TransactionResult> {
            let mut count = self.count.lock().unwrap();
            debug!("Mocker received transaction: {:?}", tx);
            match *count {
                0 => {
                    *count += 1;
                    Some(TransactionResult::Drop)
                }
                1 => {
                    *count += 1;
                    Some(TransactionResult::signature_status_error(
                        Signature::new_unique(),
                        TransactionError::AccountInUse,
                    ))
                }
                _ => Some(TransactionResult::signature_status_success(
                    Signature::new_unique(),
                )),
            }
        }
        fn is_blockhash_valid(&self, _blockhash: &str) -> Option<bool> {
            Some(false)
        }
    }

    let mocker = Arc::new(DropFailSucceedMocker {
        count: Mutex::new(0),
    });

    let (url, handle) = utils::start(mocker).await.unwrap();
    let rpc_client = utils::create_rpc_client(&url);

    let dropped_tx = create_account_tx();
    let res_dropped =
        rpc_client.send_and_confirm_transaction(&dropped_tx).await;
    assert!(res_dropped.is_err());
    assert!(res_dropped
        .unwrap_err()
        .to_string()
        .contains("unable to confirm transaction"));

    let failed_tx = create_account_tx();
    let res_failed = rpc_client.send_and_confirm_transaction(&failed_tx).await;
    assert!(res_failed.is_err());

    let success_tx = create_account_tx();
    let res_success =
        rpc_client.send_and_confirm_transaction(&success_tx).await;
    assert!(res_success.is_ok());

    let sig_dropped = dropped_tx.get_signature();
    let sig_failed = failed_tx.get_signature();
    let sig_success = success_tx.get_signature();

    let statuses =
        sig_statuses(&rpc_client, &[*sig_dropped, *sig_failed, *sig_success])
            .await;

    assert_eq!(statuses.len(), 3);
    assert!(statuses[0].is_none());
    assert!(statuses[1].as_ref().unwrap().err.is_some());
    assert_eq!(statuses[2], successful_transaction_status());

    utils::stop(handle).await;
}

#[tokio::test]
async fn test_mixed_transaction_handling_with_proxy_fallback() {
    let source_keypair = Keypair::new();
    let dest_keypair = Keypair::new();
    let transfer_amount = 1_000_000;

    // Create a direct RPC client to development cluster for setup and verification
    let dev_rpc_client = RpcClient::new("http://127.0.0.1:7799".to_string());

    // Request airdrop to source account on development cluster
    debug!("Requesting airdrop to source account...");
    let airdrop_sig = match dev_rpc_client
        .request_airdrop(&source_keypair.pubkey(), 10_000_000)
        .await
    {
        Ok(sig) => sig,
        Err(e) => {
            panic!("Development cluster airdrop failed: {:?}", e);
        }
    };

    // Wait for airdrop to be confirmed
    for _attempt in 0..30 {
        if let Ok(status) =
            dev_rpc_client.get_signature_status(&airdrop_sig).await
        {
            if status.is_some() {
                info!("Airdrop confirmed");
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }

    // Verify source account has balance
    debug!("Verifying balance in source account...");
    let source_balance = dev_rpc_client
        .get_balance(&source_keypair.pubkey())
        .await
        .expect("Failed to get source balance");
    assert!(
        source_balance >= transfer_amount,
        "Source account doesn't have enough balance"
    );

    // Create the three different transactions
    let failed_tx = create_account_tx(); // Will be mocked to fail
    let success_tx = create_account_tx(); // Will be mocked to succeed

    // Create real transfer transaction that will go to proxy
    let recent_blockhash = dev_rpc_client
        .get_latest_blockhash()
        .await
        .expect("Failed to get recent blockhash");

    let transfer_instruction = system_instruction::transfer(
        &source_keypair.pubkey(),
        &dest_keypair.pubkey(),
        transfer_amount,
    );

    let transfer_tx = VersionedTransaction::try_new(
        VersionedMessage::V0(
            Message::try_compile(
                &source_keypair.pubkey(),
                &[transfer_instruction],
                &[],
                recent_blockhash,
            )
            .expect("Failed to compile message"),
        ),
        &[&source_keypair],
    )
    .expect("Failed to create transfer transaction");

    struct MixedTransactionMocker {
        failed_sig: Signature,
        success_sig: Signature,
        transfer_sig: Signature,
    }

    impl SoxMocker for MixedTransactionMocker {
        fn handle_transaction(
            &self,
            tx: VersionedTransaction,
        ) -> Option<TransactionResult> {
            let sig = *tx.signatures.first().unwrap();

            if sig == self.failed_sig {
                // Mock as failed
                Some(TransactionResult::signature_status_error(
                    sig,
                    TransactionError::InsufficientFundsForFee,
                ))
            } else if sig == self.success_sig {
                // Mock as successful
                Some(TransactionResult::signature_status_success(sig))
            } else if sig == self.transfer_sig {
                // Pass through to proxy (return None)
                None
            } else {
                // For any other transactions, also pass through
                None
            }
        }
    }

    let mocker = Arc::new(MixedTransactionMocker {
        failed_sig: *failed_tx.signatures.first().unwrap(),
        success_sig: *success_tx.signatures.first().unwrap(),
        transfer_sig: *transfer_tx.signatures.first().unwrap(),
    });

    debug!("Starting sox proxy...");
    let (url, handle) =
        utils::start_with_config(mocker, sox::ResponderConfig::development())
            .await
            .unwrap();
    let rpc_client = utils::create_rpc_client(&url);

    // Send all three transactions
    debug!("Sending failed transaction...");
    let failed_result = rpc_client.send_transaction(&failed_tx).await;
    assert!(
        failed_result.is_ok(),
        "Failed transaction should be accepted"
    );

    debug!("Sending success transaction...");
    let success_result = rpc_client.send_transaction(&success_tx).await;
    assert!(
        success_result.is_ok(),
        "Success transaction should be accepted"
    );

    debug!("Sending transfer transaction (proxy fallback)...");
    let transfer_result = rpc_client.send_transaction(&transfer_tx).await;
    assert!(
        transfer_result.is_ok(),
        "Transfer transaction should be accepted"
    );

    // Wait for the failed transaction to be process
    for _attempt in 0..10 {
        if let Ok(status) = rpc_client
            .get_signature_status_with_commitment(
                &transfer_tx.signatures[0],
                CommitmentConfig::processed(),
            )
            .await
        {
            if status.is_some() {
                debug!("Transfer transaction confirmed");
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // Check transaction statuses through Sox
    let statuses = sig_statuses(
        &rpc_client,
        &[
            *failed_tx.signatures.first().unwrap(),
            *success_tx.signatures.first().unwrap(),
            *transfer_tx.signatures.first().unwrap(),
        ],
    )
    .await;

    assert_eq!(statuses.len(), 3);

    // Failed transaction should show error
    let failed_status = statuses[0].as_ref().unwrap();
    assert!(
        failed_status.err.is_some(),
        "Failed transaction should have error"
    );

    // Success transaction should show success
    let success_status = statuses[1].as_ref().unwrap();
    assert!(
        success_status.err.is_none(),
        "Success transaction should have no error"
    );

    // Verify the transfer actually happened on development cluster
    debug!("Verifying account balances...");
    let dest_balance = dev_rpc_client
        .get_balance_with_commitment(
            &dest_keypair.pubkey(),
            CommitmentConfig::processed(),
        )
        .await
        .expect("Failed to get destination balance");

    assert_eq!(
        dest_balance.value, transfer_amount,
        "Destination account should have received the transferred amount"
    );

    // Also verify the source account balance decreased
    let final_source_balance = dev_rpc_client
        .get_balance_with_commitment(
            &source_keypair.pubkey(),
            CommitmentConfig::processed(),
        )
        .await
        .expect("Failed to get final source balance")
        .value;

    assert!(
        final_source_balance < source_balance,
        "Source account balance should have decreased"
    );

    utils::stop(handle).await;
}
