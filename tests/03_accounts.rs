use std::sync::Arc;

use base64::{engine::general_purpose, Engine};
use solana_account_decoder::{UiAccount, UiAccountData, UiAccountEncoding};
use solana_rpc_client_api::config::RpcAccountInfoConfig;
use solana_sdk::{
    account::Account, commitment_config::CommitmentConfig, pubkey::Pubkey, system_program,
};
use sox::mocker::SoxMocker;

mod utils;

fn create_mocked_account() -> (Account, UiAccount) {
    let owner = system_program::id();
    let account = Account {
        lamports: 100,
        data: vec![],
        owner,
        executable: false,
        rent_epoch: 0,
    };
    let ui_account_data = UiAccountData::Binary(
        general_purpose::STANDARD.encode(&account.data),
        UiAccountEncoding::Base64,
    );
    let mocked_ui_account = UiAccount {
        lamports: account.lamports,
        data: ui_account_data,
        owner: account.owner.to_string(),
        executable: account.executable,
        rent_epoch: account.rent_epoch,
        space: Some(account.data.len() as u64),
    };
    (account, mocked_ui_account)
}

#[tokio::test]
async fn test_get_account_info_mocked() {
    let existing_pubkey = Pubkey::new_unique();
    let non_existing_pubkey = Pubkey::new_unique();
    let passthrough_pubkey = Pubkey::new_unique();

    let (mocked_account, mocked_ui_account) = create_mocked_account();

    struct AccountMocker {
        existing_pubkey: Pubkey,
        non_existing_pubkey: Pubkey,
        mocked_account: UiAccount,
    }

    impl SoxMocker for AccountMocker {
        fn get_account_info(
            &self,
            pubkey_str: &str,
            _config: Option<RpcAccountInfoConfig>,
        ) -> Option<Option<UiAccount>> {
            let pubkey = Pubkey::try_from(pubkey_str).unwrap();
            if pubkey == self.existing_pubkey {
                Some(Some(self.mocked_account.clone()))
            } else if pubkey == self.non_existing_pubkey {
                Some(None)
            } else {
                None
            }
        }
    }

    let mocker = Arc::new(AccountMocker {
        existing_pubkey,
        non_existing_pubkey,
        mocked_account: mocked_ui_account.clone(),
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

    // 3. Test passthrough account
    let account = rpc_client
        .get_account_with_commitment(&passthrough_pubkey, CommitmentConfig::processed())
        .await
        .unwrap()
        .value;
    assert!(account.is_none());

    utils::stop(handle).await;
}
