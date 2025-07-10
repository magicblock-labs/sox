# Sox - Solana RPC Proxy & Mocker

Sox is a Solana RPC proxy that sits between your application and a running test validator or
devnet, allowing developers to mock return values for specific requests or actions.

This is particularly useful for simulating scenarios (especially failure cases) that are
impossible or difficult to achieve with just a test validator alone.

## Why Sox?

When developing Solana applications, you often need to test edge cases and failure scenarios:

- **Transaction Failures**: Simulate insufficient funds, invalid signatures, or custom program errors
- **Blockhash Validation**: Test expired blockhashes or invalid blockhash scenarios
- **Account States**: Mock non-existent accounts, specific account data, or ownership scenarios
- **Network Conditions**: Simulate temporary unavailability or specific error responses

Sox enables comprehensive testing by allowing you to mock specific responses while letting other requests pass through to the real validator.

## Architecture

```
Your App ──→ Sox Proxy ──→ Test Validator/Devnet
             ↕ Mock Logic
```

- **Mockable Methods**: Specific RPC methods that can return mocked responses
- **Passthrough**: All other methods proxy directly to the configured cluster
- **Fallback Logic**: Mocked methods can selectively return `None` to fall back to real responses

## Complete Mock Implementation Example

```rust
use std::collections::HashMap;
use solana_rpc_client_api::config::RpcAccountInfoConfig;
use solana_rpc_client_api::response::RpcBlockhash;
use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
    signature::Signature,
    transaction::{TransactionError, VersionedTransaction},
};
use sox::mocker::{SoxMocker, TransactionResult};

pub struct ComprehensiveMocker {
    // Transaction simulation rules
    failing_signatures: HashMap<Signature, TransactionError>,
    successful_signatures: Vec<Signature>,
    dropped_signatures: Vec<Signature>,

    // Blockhash validation rules
    valid_blockhashes: Vec<String>,
    invalid_blockhashes: Vec<String>,

    // Account mocking rules
    mocked_accounts: HashMap<Pubkey, Option<Account>>,

    // Latest blockhash override
    custom_blockhash: Option<RpcBlockhash>,
}

impl ComprehensiveMocker {
    pub fn new() -> Self {
        Self {
            failing_signatures: HashMap::new(),
            successful_signatures: Vec::new(),
            dropped_signatures: Vec::new(),
            valid_blockhashes: Vec::new(),
            invalid_blockhashes: Vec::new(),
            mocked_accounts: HashMap::new(),
            custom_blockhash: None,
        }
    }

    // Configuration methods
    pub fn fail_transaction(mut self, sig: Signature, error: TransactionError) -> Self {
        self.failing_signatures.insert(sig, error);
        self
    }

    pub fn succeed_transaction(mut self, sig: Signature) -> Self {
        self.successful_signatures.push(sig);
        self
    }

    pub fn drop_transaction(mut self, sig: Signature) -> Self {
        self.dropped_signatures.push(sig);
        self
    }

    pub fn mock_account(mut self, pubkey: Pubkey, account: Option<Account>) -> Self {
        self.mocked_accounts.insert(pubkey, account);
        self
    }

    pub fn set_blockhash_valid(mut self, blockhash: String, valid: bool) -> Self {
        if valid {
            self.valid_blockhashes.push(blockhash);
        } else {
            self.invalid_blockhashes.push(blockhash);
        }
        self
    }

    pub fn set_latest_blockhash(mut self, blockhash: RpcBlockhash) -> Self {
        self.custom_blockhash = Some(blockhash);
        self
    }
}

impl SoxMocker for ComprehensiveMocker {
    fn handle_transaction(&self, tx: VersionedTransaction) -> Option<TransactionResult> {
        let signature = *tx.signatures.first()?;

        // Check if transaction should be dropped (not processed at all)
        if self.dropped_signatures.contains(&signature) {
            return Some(TransactionResult::Drop);
        }

        // Check if transaction should fail with specific error
        if let Some(error) = self.failing_signatures.get(&signature) {
            return Some(TransactionResult::signature_status_error(signature, error.clone()));
        }

        // Check if transaction should succeed
        if self.successful_signatures.contains(&signature) {
            return Some(TransactionResult::signature_status_success(signature));
        }

        // Return None to pass through to actual validator
        None
    }

    fn is_blockhash_valid(&self, blockhash: &str) -> Option<bool> {
        // Check explicit valid/invalid lists first
        if self.valid_blockhashes.contains(&blockhash.to_string()) {
            return Some(true);
        }

        if self.invalid_blockhashes.contains(&blockhash.to_string()) {
            return Some(false);
        }

        // Return None to check with actual validator
        None
    }

    fn get_account_info(
        &self,
        pubkey: &str,
        _config: Option<RpcAccountInfoConfig>,
    ) -> Option<Option<Account>> {
        // Parse pubkey and check if we have a mock for it
        if let Ok(pubkey) = pubkey.parse::<Pubkey>() {
            if let Some(account_option) = self.mocked_accounts.get(&pubkey) {
                return Some(account_option.clone());
            }
        }

        // Return None to fetch from actual validator
        None
    }

    fn get_latest_blockhash(&self) -> Option<RpcBlockhash> {
        // Return custom blockhash if set, otherwise fall back to validator
        self.custom_blockhash.clone()
    }
}
```

## Usage Examples

### 1. Transaction Testing

```rust
use solana_sdk::{
    signature::Keypair,
    signer::Signer,
    system_instruction,
    transaction::{Transaction, TransactionError},
};

#[tokio::test]
async fn test_transaction_failures() {
    let payer = Keypair::new();
    let recipient = Keypair::new();

    // Create transaction
    let tx = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), 1000)],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    // Mock the transaction to fail with insufficient funds
    let mocker = ComprehensiveMocker::new()
        .fail_transaction(*tx.get_signature(), TransactionError::InsufficientFundsForFee);

    let (url, handle) = sox::start_rpc_server(
        Arc::new(mocker),
        sox::ResponderConfig::development(),
        Some(8899),
    ).await.unwrap();

    let client = RpcClient::new(format!("http://{}", url));

    // Send transaction - should succeed (accepted)
    let signature = client.send_transaction(&tx).await.unwrap();

    // Check status - should show failure
    let status = client.get_signature_status(&signature).await.unwrap();
    assert!(status.unwrap().err.is_some()); // Transaction failed as mocked
}
```

### 2. Blockhash Validation Testing

```rust
#[tokio::test]
async fn test_blockhash_scenarios() {
    let expired_hash = "11111111111111111111111111111111";
    let valid_hash = "22222222222222222222222222222222";

    let mocker = ComprehensiveMocker::new()
        .set_blockhash_valid(expired_hash.to_string(), false)
        .set_blockhash_valid(valid_hash.to_string(), true);

    let (url, handle) = sox::start_rpc_server(
        Arc::new(mocker),
        sox::ResponderConfig::development(),
        Some(8899),
    ).await.unwrap();

    let client = RpcClient::new(format!("http://{}", url));

    // Test expired blockhash
    let is_valid = client.is_blockhash_valid(
        &expired_hash.parse().unwrap(),
        CommitmentConfig::processed(),
    ).await.unwrap();
    assert!(!is_valid); // Should be invalid (mocked)

    // Test valid blockhash
    let is_valid = client.is_blockhash_valid(
        &valid_hash.parse().unwrap(),
        CommitmentConfig::processed(),
    ).await.unwrap();
    assert!(is_valid); // Should be valid (mocked)

    // Test unknown blockhash - falls back to validator
    let real_hash = client.get_latest_blockhash().await.unwrap();
    let is_valid = client.is_blockhash_valid(&real_hash, CommitmentConfig::processed()).await.unwrap();
    // Result depends on actual validator state
}
```

### 3. Account State Testing

```rust
#[tokio::test]
async fn test_account_scenarios() {
    let existing_account = Pubkey::new_unique();
    let nonexistent_account = Pubkey::new_unique();
    let system_account = Pubkey::new_unique();

    // Create mock account data
    let mock_account = Account {
        lamports: 1_000_000,
        data: vec![1, 2, 3, 4], // Custom program data
        owner: Pubkey::new_unique(), // Custom program owner
        executable: false,
        rent_epoch: 0,
    };

    let mocker = ComprehensiveMocker::new()
        .mock_account(existing_account, Some(mock_account.clone()))
        .mock_account(nonexistent_account, None); // Explicitly doesn't exist
        // system_account not mocked - will fall back to validator

    let (url, handle) = sox::start_rpc_server(
        Arc::new(mocker),
        sox::ResponderConfig::development(),
        Some(8899),
    ).await.unwrap();

    let client = RpcClient::new(format!("http://{}", url));

    // Test mocked existing account
    let account = client.get_account(&existing_account).await.unwrap();
    assert_eq!(account.lamports, 1_000_000);
    assert_eq!(account.data, vec![1, 2, 3, 4]);

    // Test mocked non-existent account
    let result = client.get_account(&nonexistent_account).await;
    assert!(result.is_err()); // Account doesn't exist (mocked)

    // Test unmocked account - falls back to validator
    // Will return real account state from test validator
    let system_result = client.get_account(&system_program::id()).await;
    // Result depends on actual validator state
}
```

### 4. Latest Blockhash Override

```rust
#[tokio::test]
async fn test_custom_blockhash() {
    use solana_rpc_client_api::response::RpcBlockhash;

    let custom_blockhash = RpcBlockhash {
        blockhash: "CustomBlockhash1111111111111111111111".to_string(),
        last_valid_block_height: 12345,
    };

    let mocker = ComprehensiveMocker::new()
        .set_latest_blockhash(custom_blockhash.clone());

    let (url, handle) = sox::start_rpc_server(
        Arc::new(mocker),
        sox::ResponderConfig::development(),
        Some(8899),
    ).await.unwrap();

    let client = RpcClient::new(format!("http://{}", url));

    // Get latest blockhash - should return mocked value
    let (blockhash, height) = client.get_latest_blockhash_with_commitment(
        CommitmentConfig::processed()
    ).await.unwrap();

    assert_eq!(blockhash.to_string(), custom_blockhash.blockhash);
    assert_eq!(height, custom_blockhash.last_valid_block_height);
}
```

### 5. Mixed Behavior Testing

```rust
#[tokio::test]
async fn test_mixed_mock_and_real() {
    let mock_pubkey = Pubkey::new_unique();
    let mock_signature = Signature::new_unique();

    let mocker = ComprehensiveMocker::new()
        .mock_account(mock_pubkey, None) // This account doesn't exist
        .fail_transaction(mock_signature, TransactionError::InvalidAccountIndex);
        // All other requests pass through to validator

    let (url, handle) = sox::start_rpc_server(
        Arc::new(mocker),
        sox::ResponderConfig::development(),
        Some(8899),
    ).await.unwrap();

    let client = RpcClient::new(format!("http://{}", url));

    // Mock behavior: account doesn't exist
    let result = client.get_account(&mock_pubkey).await;
    assert!(result.is_err());

    // Real behavior: system program exists on validator
    let system_account = client.get_account(&system_program::id()).await;
    assert!(system_account.is_ok());

    // Real behavior: get actual latest blockhash from validator
    let (real_blockhash, _) = client.get_latest_blockhash_with_commitment(
        CommitmentConfig::processed()
    ).await.unwrap();
    // This returns real data from the test validator
}
```

## Configuration Options

```rust
// No proxy - mock-only mode
let config = ResponderConfig::noproxy();

// Development cluster (localhost:7799)
let config = ResponderConfig::development();

// Devnet
let config = ResponderConfig::devnet();
```

## Best Practices

1. **Selective Mocking**: Only mock what you need to test - let other requests pass through
2. **Realistic Data**: Use realistic account data and error scenarios
3. **Cleanup**: Always stop the Sox server after tests
4. **Isolation**: Use unique ports for parallel test execution
5. **Documentation**: Comment your mock logic to explain the test scenarios

## Integration with Test Suites

```rust
// Test utility for easy Sox setup
pub async fn setup_sox_with_mocker<M: SoxMocker + 'static>(
    mocker: M
) -> (RpcClient, ServerHandle) {
    let (url, handle) = start_rpc_server(
        Arc::new(mocker),
        ResponderConfig::development(),
        None, // Random port
    ).await.unwrap();

    let client = RpcClient::new(format!("http://{}", url));
    (client, handle)
}

#[tokio::test]
async fn my_test() {
    let mocker = MyMocker::new()
        .fail_transaction(some_signature, TransactionError::InvalidAccountIndex);

    let (client, handle) = setup_sox_with_mocker(mocker).await;

    // Your test logic here

    handle.stop().unwrap();
    handle.stopped().await;
}
```

Sox enables comprehensive Solana application testing by bridging the gap between controlled mocking and real validator behavior, giving you the best of both worlds for thorough test coverage.
