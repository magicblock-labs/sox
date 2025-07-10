use std::sync::Arc;

use solana_rpc_client_api::config::RpcAccountInfoConfig;
use solana_sdk::{
    account::Account, commitment_config::CommitmentConfig, pubkey::Pubkey, system_program,
};
use sox::mocker::SoxMocker;

mod utils;

fn create_mocked_account() -> Account {
    let owner = system_program::id();

    Account {
        lamports: 100,
        data: vec![],
        owner,
        executable: false,
        rent_epoch: 0,
    }
}

#[tokio::test]
async fn test_get_account_info_mocked_and_non_existing() {
    let existing_pubkey = Pubkey::new_unique();
    let non_existing_pubkey = Pubkey::new_unique();

    struct AccountMocker {
        existing_pubkey: Pubkey,
        non_existing_pubkey: Pubkey,
        mocked_account: Account,
    }

    impl SoxMocker for AccountMocker {
        fn get_account_info(
            &self,
            pubkey_str: &str,
            _config: Option<RpcAccountInfoConfig>,
        ) -> Option<Option<Account>> {
            let pubkey = Pubkey::try_from(pubkey_str).unwrap();
            if pubkey == self.existing_pubkey {
                // Return the mocked account for the existing pubkey
                Some(Some(self.mocked_account.clone()))
            } else if pubkey == self.non_existing_pubkey {
                // Simulate non existing account by returning a None value
                Some(None)
            } else {
                // Signal we want to pass through by returning nothing
                None
            }
        }
    }

    let mocked_account = create_mocked_account();
    let mocker = Arc::new(AccountMocker {
        existing_pubkey,
        non_existing_pubkey,
        mocked_account: mocked_account.clone(),
    });

    let (url, handle) = utils::start(mocker).await.unwrap();
    let rpc_client = utils::create_rpc_client(&url);

    // 1. Test existing account
    let account = rpc_client
        .get_account_with_commitment(&existing_pubkey, CommitmentConfig::processed())
        .await
        .unwrap()
        .value
        .unwrap();
    assert_eq!(account.lamports, mocked_account.lamports);
    assert_eq!(account.owner, mocked_account.owner);

    // 2. Test non-existing account
    let account = rpc_client
        .get_account_with_commitment(&non_existing_pubkey, CommitmentConfig::processed())
        .await
        .unwrap()
        .value;
    assert!(account.is_none());

    utils::stop(handle).await;
}

#[tokio::test]
async fn test_get_multiple_accounts_mixed() {
    let existing_pubkey1 = Pubkey::new_unique();
    let existing_pubkey2 = Pubkey::new_unique();
    let non_existing_pubkey = Pubkey::new_unique();

    struct MultipleAccountsMocker {
        existing_pubkey1: Pubkey,
        existing_pubkey2: Pubkey,
        non_existing_pubkey: Pubkey,
        mocked_account1: Account,
        mocked_account2: Account,
    }

    impl SoxMocker for MultipleAccountsMocker {
        fn get_account_info(
            &self,
            pubkey_str: &str,
            _config: Option<RpcAccountInfoConfig>,
        ) -> Option<Option<Account>> {
            let pubkey = Pubkey::try_from(pubkey_str).unwrap();
            if pubkey == self.existing_pubkey1 {
                Some(Some(self.mocked_account1.clone()))
            } else if pubkey == self.existing_pubkey2 {
                Some(Some(self.mocked_account2.clone()))
            } else if pubkey == self.non_existing_pubkey {
                Some(None)
            } else {
                // If we encounter an unknown pubkey, signal that we want to pass through
                None
            }
        }
    }

    let mocked_account1 = create_mocked_account();
    let mut mocked_account2 = create_mocked_account();
    mocked_account2.lamports = 200; // Make it different from the first account

    let mocker = Arc::new(MultipleAccountsMocker {
        existing_pubkey1,
        existing_pubkey2,
        non_existing_pubkey,
        mocked_account1: mocked_account1.clone(),
        mocked_account2: mocked_account2.clone(),
    });

    let (url, handle) = utils::start(mocker).await.unwrap();
    let rpc_client = utils::create_rpc_client(&url);

    // Test a mix of existing and non-existing accounts
    let pubkeys = vec![existing_pubkey1, non_existing_pubkey, existing_pubkey2];
    let accounts = rpc_client
        .get_multiple_accounts_with_commitment(&pubkeys, CommitmentConfig::processed())
        .await
        .unwrap()
        .value;

    assert_eq!(accounts.len(), 3);

    // First account should exist
    assert!(accounts[0].is_some());
    let account1 = accounts[0].as_ref().unwrap();
    assert_eq!(account1.lamports, mocked_account1.lamports);
    assert_eq!(account1.owner, mocked_account1.owner);

    // Second account should not exist
    assert!(accounts[1].is_none());

    // Third account should exist
    assert!(accounts[2].is_some());
    let account2 = accounts[2].as_ref().unwrap();
    assert_eq!(account2.lamports, mocked_account2.lamports);
    assert_eq!(account2.owner, mocked_account2.owner);

    utils::stop(handle).await;
}
