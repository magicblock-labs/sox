use std::{str::FromStr, sync::Arc};

use solana_rpc_client_api::config::RpcAccountInfoConfig;
use solana_sdk::{
    account::Account, commitment_config::CommitmentConfig, pubkey::Pubkey,
    system_program,
};
use sox::mocker::{SoxMocker, SoxPassThrough};

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
        .get_account_with_commitment(
            &existing_pubkey,
            CommitmentConfig::processed(),
        )
        .await
        .unwrap()
        .value
        .unwrap();
    assert_eq!(account.lamports, mocked_account.lamports);
    assert_eq!(account.owner, mocked_account.owner);

    // 2. Test non-existing account
    let account = rpc_client
        .get_account_with_commitment(
            &non_existing_pubkey,
            CommitmentConfig::processed(),
        )
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
        .get_multiple_accounts_with_commitment(
            &pubkeys,
            CommitmentConfig::processed(),
        )
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

#[tokio::test]
async fn test_get_account_info_fallback_to_proxy() {
    let mocker = Arc::new(SoxPassThrough);
    let (url, handle) =
        utils::start_with_config(mocker, sox::ResponderConfig::development())
            .await
            .unwrap();
    let rpc_client = utils::create_rpc_client(&url);

    // Use a well-known system program account that should exist on development
    let system_program_pubkey = system_program::id();

    // This should work by falling back to the remote proxy
    let result = rpc_client
        .get_account_with_commitment(
            &system_program_pubkey,
            CommitmentConfig::processed(),
        )
        .await;

    // Should succeed (not panic or return an error about no proxy)
    assert!(
        result.is_ok(),
        "getAccountInfo should fall back to proxy when mock returns None"
    );

    let account_response = result.unwrap();
    // System program account should exist on development cluster
    assert!(
        account_response.value.is_some(),
        "System program account should exist on development"
    );

    let account = account_response.value.unwrap();
    // System program should be executable
    assert!(account.executable, "System program should be executable");
    // System program should be owned by the Native Loader
    assert_eq!(account.owner, solana_sdk::native_loader::id());

    utils::stop(handle).await;
}

#[tokio::test]
async fn test_get_multiple_accounts_fallback_to_proxy() {
    let mocker = Arc::new(SoxPassThrough);
    let (url, handle) =
        utils::start_with_config(mocker, sox::ResponderConfig::development())
            .await
            .unwrap();
    let rpc_client = utils::create_rpc_client(&url);

    // Use well-known accounts that should exist on development
    let pubkeys = vec![
        system_program::id(),
        solana_sdk::sysvar::rent::id(),
        solana_sdk::sysvar::clock::id(),
    ];

    // This should work by falling back to the development proxy
    let result = rpc_client
        .get_multiple_accounts_with_commitment(
            &pubkeys,
            CommitmentConfig::processed(),
        )
        .await;

    // Should succeed (not panic or return an error about no proxy)
    assert!(
        result.is_ok(),
        "getMultipleAccounts should fall back to proxy when mock returns None"
    );

    let accounts_response = result.unwrap();
    let accounts = accounts_response.value;

    // Should return 3 accounts
    assert_eq!(accounts.len(), 3);

    // System program should exist and be executable
    assert!(
        accounts[0].is_some(),
        "System program should exist on development"
    );
    let system_program = accounts[0].as_ref().unwrap();
    assert!(
        system_program.executable,
        "System program should be executable"
    );
    assert_eq!(system_program.owner, solana_sdk::native_loader::id());

    // Rent sysvar should exist
    assert!(
        accounts[1].is_some(),
        "Rent sysvar should exist on development"
    );
    let rent_sysvar = accounts[1].as_ref().unwrap();
    assert_eq!(rent_sysvar.owner, solana_sdk::sysvar::id());

    // Clock sysvar should exist
    assert!(
        accounts[2].is_some(),
        "Clock sysvar should exist on development"
    );
    let clock_sysvar = accounts[2].as_ref().unwrap();
    assert_eq!(clock_sysvar.owner, solana_sdk::sysvar::id());

    utils::stop(handle).await;
}

#[tokio::test]
async fn test_get_multiple_accounts_mixed_mock_and_proxy() {
    // Create test accounts for mocking
    let first_account = create_mocked_account();
    let third_account = Account {
        lamports: 9999,
        data: vec![1, 2, 3, 4],
        owner: Pubkey::new_unique(),
        executable: false,
        rent_epoch: 0,
    };

    // Create pubkeys for the test
    let first_pubkey = Pubkey::new_unique();
    let second_pubkey = Pubkey::new_unique(); // Will be mocked as non-existent
    let rent_pubkey = solana_sdk::sysvar::rent::id(); // Will fall back to proxy
    let clock_pubkey = solana_sdk::sysvar::clock::id(); // Will fall back to proxy
    let third_pubkey = Pubkey::new_unique();

    struct MixedAccountMocker {
        first_pubkey: Pubkey,
        second_pubkey: Pubkey,
        third_pubkey: Pubkey,
        first_account: Account,
        third_account: Account,
    }

    impl SoxMocker for MixedAccountMocker {
        fn get_account_info(
            &self,
            pubkey: &str,
            _config: Option<RpcAccountInfoConfig>,
        ) -> Option<Option<Account>> {
            let pubkey = Pubkey::from_str(pubkey).ok()?;

            if pubkey == self.first_pubkey {
                Some(Some(self.first_account.clone()))
            } else if pubkey == self.second_pubkey {
                // Return mocked None for Second (account doesn't exist)
                Some(None)
            } else if pubkey == self.third_pubkey {
                Some(Some(self.third_account.clone()))
            } else {
                // For Rent and Clock, return None to fall back to proxy
                None
            }
        }
    }

    let mocker = Arc::new(MixedAccountMocker {
        first_pubkey,
        second_pubkey,
        third_pubkey,
        first_account: first_account.clone(),
        third_account: third_account.clone(),
    });

    let (url, handle) =
        utils::start_with_config(mocker, sox::ResponderConfig::development())
            .await
            .unwrap();
    let rpc_client = utils::create_rpc_client(&url);

    // Request accounts in the specified order: First, Second, Rent, Clock, Third
    let pubkeys = vec![
        first_pubkey,
        second_pubkey,
        rent_pubkey,
        clock_pubkey,
        third_pubkey,
    ];

    let result = rpc_client
        .get_multiple_accounts_with_commitment(
            &pubkeys,
            CommitmentConfig::processed(),
        )
        .await;

    assert!(
        result.is_ok(),
        "getMultipleAccounts should handle mixed mock and proxy requests"
    );

    let accounts_response = result.unwrap();
    let accounts = accounts_response.value;

    // Should return 5 accounts
    assert_eq!(accounts.len(), 5);

    // First account should be the mocked account
    assert!(accounts[0].is_some(), "First account should exist (mocked)");
    let first_result = accounts[0].as_ref().unwrap();
    assert_eq!(first_result.lamports, first_account.lamports);
    assert_eq!(first_result.owner, first_account.owner);

    // Second account should not exist (mocked as None)
    assert!(
        accounts[1].is_none(),
        "Second account should not exist (mocked as None)"
    );

    // Rent sysvar should exist (from proxy)
    assert!(
        accounts[2].is_some(),
        "Rent sysvar should exist (from proxy)"
    );
    let rent_result = accounts[2].as_ref().unwrap();
    assert_eq!(rent_result.owner, solana_sdk::sysvar::id());

    // Clock sysvar should exist (from proxy)
    assert!(
        accounts[3].is_some(),
        "Clock sysvar should exist (from proxy)"
    );
    let clock_result = accounts[3].as_ref().unwrap();
    assert_eq!(clock_result.owner, solana_sdk::sysvar::id());

    // Third account should be the mocked account
    assert!(accounts[4].is_some(), "Third account should exist (mocked)");
    let third_result = accounts[4].as_ref().unwrap();
    assert_eq!(third_result.lamports, third_account.lamports);
    assert_eq!(third_result.owner, third_account.owner);

    utils::stop(handle).await;
}
