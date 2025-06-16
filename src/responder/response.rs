use solana_rpc_client_api::response::{Response, RpcResponseContext};

pub fn response_with_context<T>(value: T) -> Response<T> {
    Response {
        context: RpcResponseContext::new(0),
        value,
    }
}
