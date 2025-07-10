use base64::Engine;
use solana_account_decoder::UiAccount;
use solana_sdk::account::Account;

pub(crate) fn into_account_info(account: Account) -> UiAccount {
    UiAccount {
        lamports: account.lamports,
        data: solana_account_decoder::UiAccountData::Binary(
            base64::engine::general_purpose::STANDARD.encode(&account.data),
            solana_account_decoder::UiAccountEncoding::Base64,
        ),
        owner: account.owner.to_string(),
        executable: account.executable,
        rent_epoch: account.rent_epoch,
        space: Some(account.data.len() as u64),
    }
}
