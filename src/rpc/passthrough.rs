use crate::responder::ResponderRpc;
use jsonrpsee::{
    core::RegisterMethodError,
    types::{ErrorObjectOwned, Params},
    RpcModule,
};
use log::*;
use serde::de::DeserializeOwned;
use solana_account_decoder::{parse_token::UiTokenAmount, UiAccount};
use solana_rpc_client_api::response::{
    OptionalContext, Response as RpcResponse, RpcAccountBalance, RpcBlockCommitment,
    RpcBlockProduction, RpcBlockhash, RpcConfirmedTransactionStatusWithSignature, RpcContactInfo,
    RpcIdentity, RpcInflationGovernor, RpcInflationRate, RpcInflationReward, RpcKeyedAccount,
    RpcLeaderSchedule, RpcPerfSample, RpcPrioritizationFee, RpcSimulateTransactionResult,
    RpcSnapshotSlotInfo, RpcSupply, RpcTokenAccountBalance, RpcVersionInfo, RpcVoteAccountStatus,
};
use solana_sdk::{
    clock::{Slot, UnixTimestamp},
    epoch_info::EpochInfo,
    epoch_schedule::EpochSchedule,
};
use solana_transaction_status::{TransactionStatus, UiConfirmedBlock};

// -----------------
// Solana Types
// -----------------
// Copied here instead of depending on large crates
const MAX_LOCKOUT_HISTORY: usize = 31;
type BlockCommitmentArray = [u64; MAX_LOCKOUT_HISTORY + 1];

// -----------------
// register_passthrough_methods
// -----------------
async fn passthrough_impl<R: DeserializeOwned>(
    method: &str,
    params: Params<'static>,
    rpc: &ResponderRpc,
) -> Result<R, ErrorObjectOwned> {
    rpc.handle_request(method, params).await
}

pub fn register_passthrough_methods(
    module: &mut RpcModule<ResponderRpc>,
) -> Result<(), RegisterMethodError> {
    macro_rules! passthrough {
        ($method:literal, $return_type:ty) => {
            module.register_async_method($method, |params, rpc| async move {
                debug!("{}", $method);
                trace!("{:#?}", params);
                passthrough_impl::<$return_type>($method, params, &rpc).await
            })?;
        };
    }

    passthrough!("getAccountInfo", RpcResponse<Option<UiAccount>>);
    passthrough!("getBalance", RpcResponse<u64>);
    passthrough!("getBlock", Option<UiConfirmedBlock>);
    passthrough!(
        "getBlockCommitment",
        RpcBlockCommitment<BlockCommitmentArray>
    );
    passthrough!("getBlockHeight", u64);
    passthrough!("getBlockProduction", RpcResponse<RpcBlockProduction>);
    passthrough!("getBlockTime", Option<UnixTimestamp>);
    passthrough!("getBlocks", Vec<Slot>);
    passthrough!("getBlocksWithLimit", Vec<Slot>);
    passthrough!("getClusterNodes", Vec<RpcContactInfo>);
    passthrough!("getEpochInfo", EpochInfo);
    passthrough!("getEpochSchedule", EpochSchedule);
    passthrough!("getFeeForMessage", RpcResponse<Option<u64>>);
    passthrough!("getFirstAvailableBlock", Slot);
    passthrough!("getGenesisHash", String);
    passthrough!("getHealth", String);
    passthrough!("getHighestSnapshotSlot", RpcSnapshotSlotInfo);
    passthrough!("getIdentity", RpcIdentity);
    passthrough!("getInflationGovernor", RpcInflationGovernor);
    passthrough!("getInflationRate", RpcInflationRate);
    passthrough!("getInflationReward", Vec<Option<RpcInflationReward>>);
    passthrough!("getLargestAccounts", RpcResponse<Vec<RpcAccountBalance>>);
    // TODO: guide Ephem (ephemeral validator should match the on chain blockhash)
    passthrough!("getLatestBlockhash", RpcResponse<RpcBlockhash>);
    passthrough!("getLeaderSchedule", Option<RpcLeaderSchedule>);
    passthrough!("getMaxRetransmitSlot", Slot);
    passthrough!("getMaxShredInsertSlot", Slot);
    passthrough!("getMinimumBalanceForRentExemption", u64);
    passthrough!("getMultipleAccounts", RpcResponse<Vec<Option<UiAccount>>>);
    passthrough!("getProgramAccounts", OptionalContext<Vec<RpcKeyedAccount>>);
    passthrough!("getRecentPerformanceSamples", Vec<RpcPerfSample>);
    passthrough!("getRecentPrioritizationFees", Vec<RpcPrioritizationFee>);
    passthrough!(
        "getSignatureStatuses",
        RpcResponse<Vec<Option<TransactionStatus>>>
    );
    passthrough!(
        "getSignaturesForAddress",
        Vec<RpcConfirmedTransactionStatusWithSignature>
    );
    passthrough!("getSlot", Slot);
    passthrough!("getSlotLeader", String);
    passthrough!("getSlotLeaders", Vec<String>);
    passthrough!("getStakeMinimumDelegation", RpcResponse<u64>);
    passthrough!("getSupply", RpcResponse<RpcSupply>);
    passthrough!("getTokenAccountBalance", RpcResponse<UiTokenAmount>);
    passthrough!(
        "getTokenAccountsByDelegate",
        RpcResponse<Vec<RpcKeyedAccount>>
    );
    passthrough!("getTokenAccountsByOwner", RpcResponse<Vec<RpcKeyedAccount>>);
    passthrough!(
        "getTokenLargestAccounts",
        RpcResponse<Vec<RpcTokenAccountBalance>>
    );
    passthrough!("getTokenSupply", RpcResponse<UiTokenAmount>);
    passthrough!("getTransactionCount", u64);
    passthrough!("getVersion", RpcVersionInfo);
    passthrough!("getVoteAccounts", RpcVoteAccountStatus);
    passthrough!("isBlockhashValid", RpcResponse<bool>);
    passthrough!("minimumLedgerSlot", Slot);
    passthrough!("requestAirdrop", String);
    passthrough!(
        "simulateTransaction",
        RpcResponse<RpcSimulateTransactionResult>
    );

    Ok(())
}
