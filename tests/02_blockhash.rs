use log::*;
use std::sync::Arc;

use solana_rpc_client_api::response::RpcBlockhash;
use solana_sdk::{commitment_config::CommitmentConfig, hash::Hash};
use sox::mocker::{SoxMocker, SoxPassThrough};
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

#[tokio::test]
async fn test_get_latest_blockhash_fallback_to_proxy() {
    let mocker = Arc::new(SoxPassThrough);
    let (url, handle) = utils::start_with_config(mocker, sox::ResponderConfig::development())
        .await
        .unwrap();
    let rpc_client = utils::create_rpc_client(&url);

    let result = rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::processed())
        .await;

    debug!(
        "get_latest_blockhash_with_commitment returned: {:?}",
        result
    );

    assert!(
        result.is_ok(),
        "getLatestBlockhash should fall back to proxy when mock returns None"
    );

    let (blockhash, _) = result.unwrap();
    assert!(
        !blockhash.to_string().is_empty(),
        "Blockhash should not be empty"
    );

    utils::stop(handle).await;
}

#[tokio::test]
async fn test_is_blockhash_valid_fallback_to_proxy() {
    let mocker = Arc::new(SoxPassThrough);
    let (url, handle) = utils::start_with_config(mocker, sox::ResponderConfig::development())
        .await
        .unwrap();
    let rpc_client = utils::create_rpc_client(&url);

    let (blockhash, _) = rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::processed())
        .await
        .unwrap();

    let result = rpc_client
        .is_blockhash_valid(&blockhash, CommitmentConfig::processed())
        .await;

    debug!("is_blockhash_valid returned: {:?}", result);

    assert!(
        result.is_ok(),
        "isBlockhashValid should fall back to proxy when mock returns None"
    );

    assert!(result.unwrap(), "Recent blockhash should be valid");

    utils::stop(handle).await;
}

#[tokio::test]
async fn test_is_blockhash_valid_mixed_mock_and_proxy() {
    // Create a specific invalid blockhash that will be mocked
    let invalid_blockhash = Hash::new_unique();

    struct MixedBlockhashMocker {
        invalid_blockhash: Hash,
    }
    impl SoxMocker for MixedBlockhashMocker {
        fn is_blockhash_valid(&self, blockhash: &str) -> Option<bool> {
            if blockhash == self.invalid_blockhash.to_string() {
                // Mock this specific hash as invalid
                Some(false)
            } else {
                // For all other hashes, return None to fall back to proxy
                None
            }
        }
    }

    let mocker = Arc::new(MixedBlockhashMocker { invalid_blockhash });
    let (url, handle) = utils::start_with_config(mocker, sox::ResponderConfig::devnet())
        .await
        .unwrap();
    let rpc_client = utils::create_rpc_client(&url);

    // Test 1: The specific mocked hash should return false
    let result = rpc_client
        .is_blockhash_valid(&invalid_blockhash, CommitmentConfig::processed())
        .await
        .unwrap();

    assert!(!result, "Mocked invalid blockhash should return false");

    // Get a real blockhash from remote (should be valid via proxy)
    let (real_blockhash, _) = rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::processed())
        .await
        .unwrap();

    // The real blockhash should return true (via proxy fallback)
    let result = rpc_client
        .is_blockhash_valid(&real_blockhash, CommitmentConfig::processed())
        .await
        .unwrap();

    assert!(
        result,
        "Real blockhash from devnet should be valid via proxy fallback"
    );

    utils::stop(handle).await;
}
