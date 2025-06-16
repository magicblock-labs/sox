use jsonrpsee::types::{ErrorCode, ErrorObject, ErrorObjectOwned};
use serde::Serialize;
use thiserror::Error;

#[allow(unused)]
pub fn invalid_params(msg: String) -> ErrorObjectOwned {
    ErrorObject::owned(ErrorCode::InvalidParams.code(), msg, None::<String>)
}

#[derive(Debug)]
pub enum ServerErrorCode {
    RpcClientError = 1,
}

pub fn server_error(msg: String, code: ServerErrorCode) -> ErrorObjectOwned {
    ErrorObject::owned(code as i32, msg, None::<String>)
}

#[allow(unused)]
pub fn server_error_with_data<S: Serialize>(
    msg: String,
    code: ServerErrorCode,
    data: S,
) -> ErrorObjectOwned {
    ErrorObject::owned(code as i32, msg, Some(data))
}

pub type ResponderRpcResult<T> = Result<T, ResponderRpcError>;

#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
pub enum ResponderRpcError {
    #[error("JsonRpcRegisterMethodError")]
    JsonRpcRegisterMethodError(#[from] jsonrpsee::core::RegisterMethodError),
    #[error("JsonRpcClientError")]
    JsonRpcClientError(#[from] jsonrpsee::core::client::Error),
    #[error("StdIoError")]
    StdIoError(#[from] std::io::Error),
}
