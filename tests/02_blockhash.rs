use std::sync::Arc;

use solana_sdk::{commitment_config::CommitmentConfig, hash::Hash};
use sox::mocker::SoxMocker;
mod utils;

#[tokio::test]
async fn test_is_blockhash_valid_mocked() {
    let valid_blockhash = Hash::new_unique();
    let invalid_blockhash = Hash::new_unique();

    struct BlockhashMocker {
        valid_blockhash: Hash,
        invalid_blockhash: Hash,
    }
    impl SoxMocker for BlockhashMocker {
        fn is_blockhash_valid(&self, blockhash: &str) -> Option<bool> {
            if blockhash == self.valid_blockhash.to_string() {
                Some(true)
            } else if blockhash == self.invalid_blockhash.to_string() {
                Some(false)
            } else {
                None
            }
        }
    }

    let mocker = Arc::new(BlockhashMocker {
        valid_blockhash,
        invalid_blockhash,
    });
    let (url, handle) = utils::start(mocker).await.unwrap();
    let rpc_client = utils::create_rpc_client(&url);

    let is_valid = rpc_client
        .is_blockhash_valid(&valid_blockhash, CommitmentConfig::processed())
        .await
        .unwrap();
    assert!(is_valid);

    let is_valid = rpc_client
        .is_blockhash_valid(&invalid_blockhash, CommitmentConfig::processed())
        .await
        .unwrap();
    assert!(!is_valid);

    utils::stop(handle).await;
}
