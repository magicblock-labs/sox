pub const MAINNET: &str = "https://api.mainnet-beta.solana.com";
pub const TESTNET: &str = "https://api.testnet.solana.com";
pub const DEVNET: &str = "https://api.devnet.solana.com";
// NOTE: that the proxy defaults to running on 8899, so we need
// to use a different port by default for the test validator
pub const DEVELOPMENT: &str = "http://localhost:7799";

pub const WS_MAINNET: &str = "wss://api.mainnet-beta.solana.com/";
pub const WS_TESTNET: &str = "wss://api.testnet.solana.com/";
pub const WS_DEVNET: &str = "wss://api.devnet.solana.com/";
pub const WS_DEVELOPMENT: &str = "ws://localhost:7900";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RpcCluster {
    Mainnet,
    Testnet,
    Devnet,
    Development,
    Custom(String, String),
}

impl RpcCluster {
    pub fn url(&self) -> &str {
        match self {
            RpcCluster::Mainnet => MAINNET,
            RpcCluster::Testnet => TESTNET,
            RpcCluster::Devnet => DEVNET,
            RpcCluster::Development => DEVELOPMENT,
            RpcCluster::Custom(url, _) => url,
        }
    }

    pub fn ws_url(&self) -> &str {
        match self {
            RpcCluster::Mainnet => WS_MAINNET,
            RpcCluster::Testnet => WS_TESTNET,
            RpcCluster::Devnet => WS_DEVNET,
            RpcCluster::Development => WS_DEVELOPMENT,
            RpcCluster::Custom(_, ws_url) => ws_url,
        }
    }
}
