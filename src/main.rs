pub mod actor;
use actor::PoolActorHandler;
use alloy::primitives::Address;
use alloy::primitives::ruint::aliases::U256;
use alloy::{self, providers::ProviderBuilder};
use alloy_transport_ws::WsConnect;
use clap;
use serde;
use std::sync::Arc;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    //make proper configs
    let ws_connect = WsConnect::new("wss://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY");
    let provider = ProviderBuilder::new().connect_ws(ws_connect).await?;
    let factory_address_str = String::from("0x1F98431c8aD98523631AE4a59f267346ea31F984");
    let actor_handle = PoolActorHandler::new(Arc::new(provider), factory_address_str);
    let token_0 = String::from("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2")
        .parse::<Address>()
        .unwrap();
    let token_1 = String::from("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")
        .parse::<Address>()
        .unwrap();
    let fee = U256::from(50u64);
    if let Ok(pool) = actor_handle.find_pool(token_0, token_1, fee).await {
        println!("Pool address is {:?}", pool);
    }
    Ok(())
}
