use std::sync::Arc;

use solana_rpc_client_api::response::RpcBlockhash;
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

#[tokio::test]
async fn test_get_latest_blockhash_mocked() {
    let test_blockhash = Hash::new_unique();
    let test_block_height = 12345u64;

    struct BlockhashMocker {
        blockhash: Hash,
        block_height: u64,
    }
    impl SoxMocker for BlockhashMocker {
        fn get_latest_blockhash(&self) -> Option<RpcBlockhash> {
            Some(RpcBlockhash {
                blockhash: self.blockhash.to_string(),
                last_valid_block_height: self.block_height,
            })
        }
    }

    let mocker = Arc::new(BlockhashMocker {
        blockhash: test_blockhash,
        block_height: test_block_height,
    });
    let (url, handle) = utils::start(mocker).await.unwrap();
    let rpc_client = utils::create_rpc_client(&url);

    let (blockhash, last_valid_block_height) = rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::processed())
        .await
        .unwrap();

    assert_eq!(blockhash.to_string(), test_blockhash.to_string());
    assert_eq!(last_valid_block_height, test_block_height);

    utils::stop(handle).await;
}
