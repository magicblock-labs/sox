use crate::{
    mocker::SoxMocker,
    responder::{response::response_with_context, ResponderRpc},
};
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
// register_mockable_methods
// -----------------
pub fn register_mockable_methods<M: SoxMocker>(
    module: &mut RpcModule<ResponderRpc<M>>,
) -> Result<(), RegisterMethodError> {
    module.register_async_method("sendTransaction", |params, rpc| async move {
        debug!("sendTransaction {:#?}", params);
        rpc.handle_send_transaction(params).await
    })?;
    Ok(())
}

// -----------------
// register_passthrough_methods
// -----------------
async fn passthrough_impl<M: SoxMocker, R: DeserializeOwned>(
    method: &str,
    params: Params<'static>,
    rpc: &ResponderRpc<M>,
    default_value: Option<R>,
) -> Result<R, ErrorObjectOwned> {
    rpc.handle_request(method, params, default_value).await
}

pub fn register_passthrough_methods<M: SoxMocker>(
    module: &mut RpcModule<ResponderRpc<M>>,
) -> Result<(), RegisterMethodError> {
    macro_rules! passthrough {
        ($method:literal, $return_type:ty) => {
            module.register_async_method($method, |params, rpc| async move {
                debug!("{}", $method);
                trace!("{:#?}", params);
                passthrough_impl::<M, $return_type>($method, params, &rpc, None).await
            })?;
        };
        ($method:literal, $return_type:ty, $default_value:expr) => {
            module.register_async_method($method, |params, rpc| async move {
                debug!("{}", $method);
                trace!("{:#?}", params);
                passthrough_impl::<M, $return_type>($method, params, &rpc, Some($default_value))
                    .await
            })?;
        };
    }

    passthrough!(
        "getAccountInfo",
        RpcResponse<Option<UiAccount>>,
        response_with_context(None)
    );
    passthrough!("getBalance", RpcResponse<u64>, response_with_context(0));
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
