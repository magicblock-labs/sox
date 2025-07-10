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
