use jsonrpsee::{core::RegisterMethodError, RpcModule};
use log::*;

use crate::{mocker::SoxMocker, responder::ResponderRpc};

pub fn register_mockable_methods<M: SoxMocker>(
    module: &mut RpcModule<ResponderRpc<M>>,
) -> Result<(), RegisterMethodError> {
    module.register_async_method("sendTransaction", |params, rpc| async move {
        debug!("sendTransaction {:?}", params);
        rpc.handle_send_transaction(params).await
    })?;
    module.register_async_method("getSignatureStatuses", |params, rpc| async move {
        debug!("getSignatureStatuses {:?}", params);
        rpc.handle_get_signature_statuses(params).await
    })?;
    Ok(())
}
