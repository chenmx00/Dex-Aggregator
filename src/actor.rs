use alloy::{
    consensus::transaction::PooledTransaction,
    primitives::{Address, U256, keccak256},
    providers::{Provider, ProviderBuilder},
    rpc::{
        client::{ClientBuilder, RpcClient},
        types::{Filter, Log},
    },
    sol,
    sol_types::SolEvent,
};
use alloy_transport_ws::WsConnect;
use anyhow::anyhow;
use futures::StreamExt;
use std::sync::Arc;
use std::{collections::HashMap, ops::Add};
use tokio::sync::{RwLock, mpsc, oneshot};

sol! {
    event Swap(
        address indexed sender,
        address indexed recipient,
        int256 amount0,
        int256 amount1,
        uint256 sqrtPriceX96,
        uint128 liquidity,
        int32 tick
    );
}

#[derive(Debug, Clone)]
pub struct PoolState {
    pub address: Address,
    pub token_0: Address,
    pub token_1: Address,
    pub fee: u32,
    pub sqrt_price_x96: U256,
    pub tick: i32,
    pub liquidity: u128,
}

enum PoolUpdate {
    Swap {
        sqrt_price: U256,
        tick: i32,
        liquidity: u128,
    },
    Mint {
        tick_lower: i32,
        tick_upper: i32,
        amount: u128,
    },
    Burn {
        tick_lower: i32,
        tick_upper: i32,
        amount: u128,
    },
}

pub enum PoolMessage {
    GetPoolState {
        pool: Address,
        send_to: oneshot::Sender<Option<PoolState>>,
    },
    SimulateSwap {
        amount_in: U256,
        token_in: Address,
        pool: Address,
        send_to: oneshot::Sender<Option<U256>>,
    },
    FindPool {
        token_0: Address,
        token_1: Address,
        fee: u32,
        send_to: oneshot::Sender<Option<Address>>,
    },
}
pub struct PoolActor {
    pools: HashMap<Address, PoolState>,
    provider: Arc<dyn Provider>,
    receiver: mpsc::Receiver<PoolMessage>,
    factory_address: Address,
}

impl PoolActor {
    fn new(
        provider: Arc<dyn Provider>,
        factory_address_str: String,
        receiver: mpsc::Receiver<PoolMessage>,
    ) -> Self {
        PoolActor {
            pools: HashMap::new(),
            provider,
            receiver,
            factory_address: factory_address_str.parse::<Address>().unwrap(),
        }
    }

    async fn on_message(&self, message: PoolMessage) -> anyhow::Result<()> {
        match message {
            PoolMessage::GetPoolState { pool, send_to } => {
                let pool_state = self.get_pool_state(&pool);
                let _ = send_to.send(pool_state);
                Ok(())
            }
            PoolMessage::FindPool {
                token_0,
                token_1,
                fee,
                send_to,
            } => {
                let pool_address = self.find_pool(&token_0, &token_1, fee);
                let _ = send_to.send(pool_address);
                Ok(())
            }
            PoolMessage::SimulateSwap {
                amount_in,
                token_in,
                pool,
                send_to,
            } => {
                let token_out = self.simulate_swap(token_in, amount_in, &pool);
                let _ = send_to.send(token_out);
                Ok(())
            }
        }
    }

    fn simulate_swap(&self, token_in: Address, amount_in: U256, pool: &Address) -> Option<U256> {
        println!("{:?}", pool);
        todo!();
    }

    fn get_pool_state(&self, pool: &Address) -> Option<PoolState> {
        println!("{:?}", pool);
        todo!();
    }

    fn find_pool(&self, token_0: &Address, token_1: &Address, fee: u32) -> Option<Address> {
        self.pools
            .values()
            .find(|x| x.token_0 == *token_0 && x.token_1 == *token_1 && fee == x.fee)
            .map(|x| x.address)
    }

    async fn index_pools(&mut self) -> anyhow::Result<()> {
        todo!();
    }

    async fn sync_pools(&mut self) -> anyhow::Result<()> {
        todo!();
    }

    async fn stream_events(
        pools: Arc<RwLock<HashMap<Address, PoolState>>>,
        provider: Arc<dyn Provider>,
    ) -> anyhow::Result<()> {
        let sub = provider.subscribe_blocks().await?;
        let mut stream = sub.into_stream();
        let mut last_processed = provider.get_block_number().await?;
        let swap_sig = keccak256("Swap(address,address,int256,int256,uint160,uint128,int24)");
        let mint_sig = keccak256("Mint(address,address,int24,int24,uint128,uint256,uint256)");
        let burn_sig = keccak256("Burn(address,int24,int24,uint128,uint256,uint256)");

        while let Some(block) = stream.next().await {
            let current = block.number;
            let pool_addresses: Vec<Address> = pools.read().await.keys().copied().collect();
            if current <= last_processed {
                continue;
            }
            let filter = Filter::new()
                .address(pool_addresses)
                .from_block(last_processed + 1)
                .to_block(current);
            let mut updates: HashMap<Address, Vec<PoolUpdate>> = HashMap::new();
            let logs: Vec<Log> = provider.get_logs(&filter).await?;
            for log in logs {
                if log.topics().is_empty() {
                    continue;
                }
                let pool_addr = log.address();
                let update = match log.topic0() {
                    Some(sig) if *sig == swap_sig => PoolActor::decode_swap_with_sol(&log),
                    Some(sig) if *sig == mint_sig => PoolActor::
                    _ => Err(anyhow!("unknown update")),
                };
            }
        }
        Ok(())
    }
    
    //refactor. can put the below three functions into one
    fn decode_swap_with_sol(log: &Log) -> anyhow::Result<PoolUpdate> {
        let data = log.data();
        match Swap::decode_log_data(data) {
            Ok(res) => {
                let sqrt_price = res.sqrtPriceX96;
                let tick = res.tick;
                let liquidity = res.liquidity;
                Ok(PoolUpdate::Swap {
                    sqrt_price,
                    tick,
                    liquidity,
                })
            }
            Err(e) => Err(anyhow!("Can't decode the swap event: {:?}", e)),
        }
    }
    
    fn decode_mint_with_sol(log: &Log) -> anyhow::Result<PoolUpdate>{
        
    }

    async fn run(&mut self) -> anyhow::Result<()> {
        println!("Start indexing pools");
        if let Err(e) = self.index_pools().await {
            eprintln!("Error encountered indexing pools: {:?}", e);
            return Err(e);
        }

        if let Err(e) = self.sync_pools().await {
            eprintln!("Error encounter syncing pools: {:?}", e);
            return Err(e);
        }
        let pools = Arc::new(RwLock::new(self.pools.clone()));
        let pools_for_stream = Arc::clone(&pools);
        let provider_for_stream = Arc::clone(&self.provider);
        let _task_handle = tokio::spawn(async move {
            if let Err(e) = Self::stream_events(pools_for_stream, provider_for_stream).await {
                eprintln!("Error: {:?}", e);
            }
        });

        while let Some(msg) = self.receiver.recv().await {
            self.on_message(msg).await?;
        }
        Ok(())
    }
}

#[cfg(tests)]
mod tests {
    use crate::actor;

    use super::*;

    #[test]
    fn test_find_pool() {
        let (tx, rx) = mpsc::channel::new();
        let ws_connect = WsConnect::new("wss://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY");
        let provider = ProviderBuilder::new().connect_ws(ws_connect).await?;
        let factory_address_str = String::from("0x1F98431c8aD98523631AE4a59f267346ea31F984");
        let actor = PoolActor::new(provider, factory_address_str, rx);
        actor.find_pool();
    }
}

#[derive(Debug, Clone)]
pub struct PoolActorHandler {
    sender: mpsc::Sender<PoolMessage>,
}

impl PoolActorHandler {
    pub fn new(provider: Arc<dyn Provider>, factory_address_str: String) -> Self {
        let (tx, rx) = mpsc::channel(32);
        let mut actor = PoolActor::new(provider, factory_address_str, rx);
        tokio::spawn(async move {
            actor.run();
        });
        Self { sender: tx }
    }

    pub async fn simulate_swap(
        &self,
        amount_in: U256,
        token_in: Address,
        pool: Address,
    ) -> anyhow::Result<U256> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(PoolMessage::SimulateSwap {
                amount_in,
                token_in,
                pool,
                send_to: tx,
            })
            .await?;
        let result = rx.await.map_err(|e| anyhow!("swap error: {:?}", e))?;
        let swap_amount = result.ok_or_else(|| anyhow!("swap amount is none"))?;
        Ok(swap_amount)
    }

    pub async fn get_pool_state(&self, pool: Address) -> anyhow::Result<PoolState> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(PoolMessage::GetPoolState { pool, send_to: tx })
            .await?;
        let result = rx
            .await
            .map_err(|e| anyhow!("fetching pool state error: {:?}", e))?;
        let pool_state = result.ok_or_else(|| anyhow!("state is empty"))?;
        Ok(pool_state)
    }

    pub async fn find_pool(
        &self,
        token_0: Address,
        token_1: Address,
        fee: U256,
    ) -> anyhow::Result<Address> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(PoolMessage::FindPool {
                token_0,
                token_1,
                fee,
                send_to: tx,
            })
            .await?;
        let result = rx
            .await
            .map_err(|e| anyhow!("finding pool error: {:?}", e))?;
        let pool = result.ok_or_else(|| anyhow!("result is empty"))?;
        Ok(pool)
    }
}
