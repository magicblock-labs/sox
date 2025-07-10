//! # Sox Comprehensive Example
//!
//! This example demonstrates the full capability of Sox by:
//! 1. Starting a Solana test validator
//! 2. Starting a Sox proxy that connects to the test validator
//! 3. Creating a comprehensive mocker that overrides account info and transaction outcomes
//! 4. Running various RPC requests against the Sox proxy
//! 5. Demonstrating both mocked and proxied responses
//! 6. Cleaning up by stopping the proxy and test validator
//!
//! Run with: `cargo run --example sox`

use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::config::RpcAccountInfoConfig;
use solana_rpc_client_api::response::RpcBlockhash;
use solana_sdk::{
    account::Account,
    commitment_config::CommitmentConfig,
    hash::Hash,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    system_instruction, system_program,
    transaction::{Transaction, TransactionError, VersionedTransaction},
};

use sox::mocker::{SoxMocker, TransactionResult};
use sox::{start_rpc_server, ResponderConfig};

/// Comprehensive mocker that demonstrates all Sox capabilities
pub struct ExampleMocker {
    // Account mocking
    mocked_accounts: HashMap<Pubkey, Option<Account>>,

    // Transaction behavior
    failing_signatures: HashMap<Signature, TransactionError>,
    successful_signatures: Vec<Signature>,
    dropped_signatures: Vec<Signature>,

    // Blockhash behavior
    custom_blockhash: Option<RpcBlockhash>,
    valid_blockhashes: Vec<String>,
    invalid_blockhashes: Vec<String>,
}

impl ExampleMocker {
    pub fn new() -> Self {
        Self {
            mocked_accounts: HashMap::new(),
            failing_signatures: HashMap::new(),
            successful_signatures: Vec::new(),
            dropped_signatures: Vec::new(),
            custom_blockhash: None,
            valid_blockhashes: Vec::new(),
            invalid_blockhashes: Vec::new(),
        }
    }

    /// Mock an account to have specific data or not exist
    pub fn mock_account(
        mut self,
        pubkey: Pubkey,
        account: Option<Account>,
    ) -> Self {
        self.mocked_accounts.insert(pubkey, account);
        self
    }

    /// Make a transaction fail with a specific error
    pub fn fail_transaction(
        mut self,
        signature: Signature,
        error: TransactionError,
    ) -> Self {
        self.failing_signatures.insert(signature, error);
        self
    }

    /// Make a transaction succeed
    pub fn succeed_transaction(mut self, signature: Signature) -> Self {
        self.successful_signatures.push(signature);
        self
    }

    /// Drop a transaction (don't process it at all)
    pub fn drop_transaction(mut self, signature: Signature) -> Self {
        self.dropped_signatures.push(signature);
        self
    }

    /// Override the latest blockhash
    pub fn set_latest_blockhash(mut self, blockhash: RpcBlockhash) -> Self {
        self.custom_blockhash = Some(blockhash);
        self
    }

    /// Set blockhash validity
    pub fn set_blockhash_valid(
        mut self,
        blockhash: String,
        valid: bool,
    ) -> Self {
        if valid {
            self.valid_blockhashes.push(blockhash);
        } else {
            self.invalid_blockhashes.push(blockhash);
        }
        self
    }
}

impl SoxMocker for ExampleMocker {
    fn handle_transaction(
        &self,
        tx: VersionedTransaction,
    ) -> Option<TransactionResult> {
        let signature = *tx.signatures.first()?;

        // Check if transaction should be dropped
        if self.dropped_signatures.contains(&signature) {
            println!("🚫 Dropping transaction: {}", signature);
            return Some(TransactionResult::Drop);
        }

        // Check if transaction should fail
        if let Some(error) = self.failing_signatures.get(&signature) {
            println!(
                "❌ Failing transaction: {} with error: {:?}",
                signature, error
            );
            return Some(TransactionResult::signature_status_error(
                signature,
                error.clone(),
            ));
        }

        // Check if transaction should succeed
        if self.successful_signatures.contains(&signature) {
            println!("✅ Forcing transaction success: {}", signature);
            return Some(TransactionResult::signature_status_success(
                signature,
            ));
        }

        // For system transfers, let them pass through to the validator
        println!("🔄 Proxying transaction to validator: {}", signature);
        None
    }

    fn get_account_info(
        &self,
        pubkey: &str,
        _config: Option<RpcAccountInfoConfig>,
    ) -> Option<Option<Account>> {
        if let Ok(pubkey) = pubkey.parse::<Pubkey>() {
            if let Some(account_option) = self.mocked_accounts.get(&pubkey) {
                match account_option {
                    Some(account) => {
                        println!(
                            "🎭 Returning mocked account for {}: {} lamports",
                            pubkey, account.lamports
                        );
                    }
                    None => {
                        println!(
                            "🎭 Returning mocked non-existent account for {}",
                            pubkey
                        );
                    }
                }
                return Some(account_option.clone());
            }
        }

        // Return None to fetch from validator
        println!(
            "🔄 Proxying account info request for {} to validator",
            pubkey
        );
        None
    }

    fn get_latest_blockhash(&self) -> Option<RpcBlockhash> {
        if let Some(ref blockhash) = self.custom_blockhash {
            println!(
                "🎭 Returning mocked latest blockhash: {}",
                blockhash.blockhash
            );
            return Some(blockhash.clone());
        }

        println!("🔄 Proxying latest blockhash request to validator");
        None
    }

    fn is_blockhash_valid(&self, blockhash: &str) -> Option<bool> {
        if self.valid_blockhashes.contains(&blockhash.to_string()) {
            println!("🎭 Returning mocked valid blockhash: {}", blockhash);
            return Some(true);
        }

        if self.invalid_blockhashes.contains(&blockhash.to_string()) {
            println!("🎭 Returning mocked invalid blockhash: {}", blockhash);
            return Some(false);
        }

        println!(
            "🔄 Proxying blockhash validation for {} to validator",
            blockhash
        );
        None
    }
}

/// Start a Solana test validator with the same arguments as the run-local-devnet.sh script
fn start_test_validator() -> Result<Child, Box<dyn std::error::Error>> {
    println!("🚀 Starting Solana test validator on port 7799...");

    let child = Command::new("solana-test-validator")
        .args(&["--log", "--rpc-port", "7799", "-r"])
        .stdout(Stdio::null()) // Suppress output to keep example clean
        .stderr(Stdio::piped()) // Capture stderr for debugging
        .spawn()?;

    // Give the validator time to start
    std::thread::sleep(Duration::from_secs(3));

    println!("✅ Test validator started");
    Ok(child)
}

/// Wait for the test validator to be ready
async fn wait_for_validator() -> Result<(), Box<dyn std::error::Error>> {
    let client = RpcClient::new("http://localhost:7799".to_string());

    println!("⏳ Waiting for test validator to be ready...");

    for i in 1..=20 {
        match client.get_health().await {
            Ok(_) => {
                println!("✅ Test validator is ready");
                return Ok(());
            }
            Err(e) => {
                if i == 20 {
                    return Err(format!("Test validator failed to start after 20 attempts. Last error: {}", e).into());
                }
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("🎯 Sox Comprehensive Example");
    println!("==============================\n");

    // Step 1: Start Solana test validator
    let mut validator = start_test_validator()?;
    wait_for_validator().await?;

    // Create some test accounts and transactions
    let mock_account_pubkey = Pubkey::new_unique();
    let nonexistent_account_pubkey = Pubkey::new_unique();
    let fake_signature = Signature::new_unique();
    let dropped_signature = Signature::new_unique();

    // Create valid hash strings
    let custom_blockhash = Hash::new_unique();
    let invalid_blockhash = Hash::new_unique();
    let valid_blockhash = Hash::new_unique();

    // Create mock account data
    let mock_account = Account {
        lamports: 5_000_000,         // 5 SOL
        data: vec![1, 2, 3, 4, 5],   // Custom program data
        owner: Pubkey::new_unique(), // Custom program
        executable: false,
        rent_epoch: 0,
    };

    // Step 2: Create comprehensive mocker
    let mocker = ExampleMocker::new()
        .mock_account(mock_account_pubkey, Some(mock_account.clone()))
        .mock_account(nonexistent_account_pubkey, None)
        .fail_transaction(
            fake_signature,
            TransactionError::InsufficientFundsForFee,
        )
        .drop_transaction(dropped_signature)
        .set_latest_blockhash(RpcBlockhash {
            blockhash: custom_blockhash.to_string(),
            last_valid_block_height: 12345,
        })
        .set_blockhash_valid(invalid_blockhash.to_string(), false)
        .set_blockhash_valid(valid_blockhash.to_string(), true);

    // Step 3: Start Sox proxy
    println!("🎭 Starting Sox proxy server...");
    let (url, handle) = start_rpc_server(
        Arc::new(mocker),
        ResponderConfig::development(), // Points to localhost:7799
        Some(8899),                     // Sox listens on port 8899
    )
    .await?;

    println!("✅ Sox proxy running at http://{}", url);
    println!("🔗 Proxying to test validator at localhost:7799\n");

    // Step 4: Create RPC client pointing to Sox
    let client = RpcClient::new(format!("http://{}", url));

    println!("🔧 Running RPC requests through Sox proxy...\n");

    // Test 1: Get account info - mocked account
    println!("📋 Test 1: Get mocked account info");
    match client.get_account(&mock_account_pubkey).await {
        Ok(account) => {
            println!(
                "   ✅ Mocked account found: {} lamports, {} bytes data",
                account.lamports,
                account.data.len()
            );
        }
        Err(e) => {
            println!("   ❌ Error: {}", e);
        }
    }

    // Test 2: Get account info - non-existent mocked account
    println!("\n📋 Test 2: Get non-existent mocked account");
    match client.get_account(&nonexistent_account_pubkey).await {
        Ok(_) => {
            println!("   ❌ Unexpected: Account should not exist");
        }
        Err(_) => {
            println!("   ✅ Correctly returned account not found (mocked)");
        }
    }

    // Test 3: Get account info - proxied to validator (system program)
    println!("\n📋 Test 3: Get real account from validator");
    match client.get_account(&system_program::id()).await {
        Ok(account) => {
            println!("   ✅ Real system program account: {} lamports (from validator)",
                account.lamports);
        }
        Err(e) => {
            println!("   ❌ Error: {}", e);
        }
    }

    // Test 4: Get latest blockhash - mocked
    println!("\n🧱 Test 4: Get mocked latest blockhash");
    match client
        .get_latest_blockhash_with_commitment(CommitmentConfig::processed())
        .await
    {
        Ok((blockhash, height)) => {
            println!(
                "   ✅ Mocked blockhash: {} at height {}",
                blockhash, height
            );
        }
        Err(e) => {
            println!("   ❌ Error: {}", e);
        }
    }

    // Test 5: Validate blockhash - mocked invalid
    println!("\n🧱 Test 5: Validate mocked invalid blockhash");
    match client
        .is_blockhash_valid(&invalid_blockhash, CommitmentConfig::processed())
        .await
    {
        Ok(valid) => {
            println!("   ✅ Mocked invalid blockhash validation: {}", valid);
        }
        Err(e) => {
            println!("   ❌ Error: {}", e);
        }
    }

    // Test 6: Validate blockhash - mocked valid
    println!("\n🧱 Test 6: Validate mocked valid blockhash");
    match client
        .is_blockhash_valid(&valid_blockhash, CommitmentConfig::processed())
        .await
    {
        Ok(valid) => {
            println!("   ✅ Mocked valid blockhash validation: {}", valid);
        }
        Err(e) => {
            println!("   ❌ Error: {}", e);
        }
    }

    // Test 7: Real system transfer transaction (proxied to validator)
    println!("\n💸 Test 7: Real system transfer transaction");

    // Create keypairs for the transfer
    let payer = Keypair::new();
    let recipient = Keypair::new();

    // First fund the payer account
    let (real_blockhash, _) = client
        .get_latest_blockhash_with_commitment(CommitmentConfig::processed())
        .await?;

    // Request airdrop for payer
    match client.request_airdrop(&payer.pubkey(), 1_000_000_000).await {
        Ok(airdrop_sig) => {
            println!("   💰 Airdrop requested: {}", airdrop_sig);

            // Wait for airdrop to confirm
            tokio::time::sleep(Duration::from_millis(1000)).await;

            // Create and send transfer transaction
            let transfer_ix = system_instruction::transfer(
                &payer.pubkey(),
                &recipient.pubkey(),
                100_000_000,
            );
            let transfer_tx = Transaction::new_signed_with_payer(
                &[transfer_ix],
                Some(&payer.pubkey()),
                &[&payer],
                real_blockhash,
            );

            match client.send_and_confirm_transaction(&transfer_tx).await {
                Ok(signature) => {
                    println!(
                        "   ✅ Transfer successful (proxied to validator): {}",
                        signature
                    );
                }
                Err(e) => {
                    println!("   ⚠️  Transfer might have failed: {} (this is expected in example)", e);
                }
            }
        }
        Err(e) => {
            println!(
                "   ⚠️  Airdrop failed: {} (this is expected in example)",
                e
            );
        }
    }

    // Test 8: Multiple accounts request - mixed behavior
    println!("\n📊 Test 8: Get multiple accounts (mixed mocked/real)");
    let accounts = vec![
        mock_account_pubkey,        // Mocked
        nonexistent_account_pubkey, // Mocked as non-existent
        system_program::id(),       // Real from validator
    ];

    match client.get_multiple_accounts(&accounts).await {
        Ok(account_results) => {
            for (i, account_opt) in account_results.iter().enumerate() {
                match account_opt {
                    Some(account) => {
                        println!(
                            "   ✅ Account {}: {} lamports",
                            i, account.lamports
                        );
                    }
                    None => {
                        println!("   ✅ Account {}: Not found", i);
                    }
                }
            }
        }
        Err(e) => {
            println!("   ❌ Error: {}", e);
        }
    }

    println!("\n🎉 Example completed successfully!");
    println!("\n🧹 Cleaning up...");

    // Step 5: Stop Sox proxy
    println!("🛑 Stopping Sox proxy...");
    handle.stop().unwrap();
    handle.stopped().await;
    println!("✅ Sox proxy stopped");

    // Step 6: Kill test validator
    println!("🛑 Stopping test validator...");
    validator.kill()?;
    validator.wait()?;
    println!("✅ Test validator stopped");

    println!("\n✨ All done! Sox example completed successfully.");

    Ok(())
}
