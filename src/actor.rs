use alloy::{
    consensus::transaction::PooledTransaction,
    primitives::{Address, I256, U256, keccak256},
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
use std::{collections::HashMap, ops::Add};
use std::{str::Bytes, sync::Arc};
use tokio::sync::{RwLock, mpsc, oneshot};

sol! {
    event Swap(
        address indexed sender,
        address indexed recipient,
        int256 amount0,
        int256 amount1,
        uint160 sqrtPriceX96,
        uint128 liquidity,
        int24 tick
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

#[derive(Debug, Clone)]
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
                    Some(sig) if *sig == mint_sig => PoolActor::decode_mint_with_sol(&log),
                    Some(sig) if *sig == burn_sig => PoolActor::decode_burn_with_sol(&log),
                    _ => Err(anyhow!("unknown update")),
                };
                if let Ok(update) = update {
                    updates
                        .entry(pool_addr)
                        .or_insert_with(|| Vec::new())
                        .push(update);
                }
                let mut pools_write = pools.write().await;
                for (addr, pool_updates) in updates.clone() {
                    if let Some(pool_state) = pools_write.get_mut(&addr) {
                        for pool_update in pool_updates {
                            match pool_update {
                                PoolUpdate::Swap {
                                    sqrt_price,
                                    tick,
                                    liquidity,
                                } => {
                                    Self::apply_swap_update(pool_state, sqrt_price, tick, liquidity)
                                }
                                PoolUpdate::Mint {
                                    tick_lower,
                                    tick_upper,
                                    amount,
                                } => Self::apply_mint_update(
                                    pool_state, tick_lower, tick_upper, amount,
                                ),
                                PoolUpdate::Burn {
                                    tick_lower,
                                    tick_upper,
                                    amount,
                                } => Self::apply_burn_update(
                                    pool_state, tick_lower, tick_upper, amount,
                                ),
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn apply_swap_update(pool_state: &mut PoolState, sqrt_price: U256, tick: i32, liquidity: u128) {
        pool_state.sqrt_price_x96 = sqrt_price;
        pool_state.tick = tick;
        pool_state.liquidity = liquidity;
    }

    fn apply_mint_update(
        pool_state: &mut PoolState,
        tick_lower: i32,
        tick_upper: i32,
        amount: u128,
    ) {
        if pool_state.tick < tick_upper && pool_state.tick > tick_lower {
            pool_state.liquidity += amount;
        }
    }

    fn apply_burn_update(
        pool_state: &mut PoolState,
        tick_lower: i32,
        tick_upper: i32,
        amount: u128,
    ) {
        if pool_state.tick < tick_upper && pool_state.tick > tick_lower {
            pool_state.liquidity -= amount;
        }
    }

    //refactor. can put the below three functions into one
    fn decode_swap_with_sol(log: &Log) -> anyhow::Result<PoolUpdate> {
        let data = log.data();
        let data_bytes: &[u8] = &data.data;
        if data_bytes.len() < 160 {
            return Err(anyhow!("Insufficient data length"));
        }
        let amount0 = I256::from_be_bytes::<32>(data_bytes[0..32].try_into()?);
        let amount1 = I256::from_be_bytes::<32>(data_bytes[32..64].try_into()?);
        let sqrt_price_x96 = U256::from_be_bytes::<32>(data_bytes[64..96].try_into()?);
        let liquidity = u128::from_be_bytes(data_bytes[112..128].try_into()?);
        let tick = {
            // Get the full 32 bytes
            let tick_word = &data_bytes[128..160];
            // Take last 4 bytes for i32 conversion
            let mut tick_bytes = [0u8; 4];
            tick_bytes[0] = if tick_word[29] & 0x80 != 0 {
                0xFF
            } else {
                0x00
            }; // Sign extension
            tick_bytes[1..4].copy_from_slice(&tick_word[29..32]); // Last 3 bytes
            i32::from_be_bytes(tick_bytes)
        };
        Ok(PoolUpdate::Swap {
            sqrt_price: sqrt_price_x96,
            tick,
            liquidity,
        })
    }

    fn decode_mint_with_sol(log: &Log) -> anyhow::Result<PoolUpdate> {
        let data = log.data();
        let data_bytes: &[u8] = &data.data;
        if data_bytes.len() < 128 {
            return Err(anyhow::anyhow!("Insufficient data length"));
        }
        let topics = log.topics();
        let owner = Address::from_slice(&topics[1][12..]);
        let tick_lower = {
            let bytes: &[u8] = topics[2].as_ref();
            // Check sign bit (int24 is 3 bytes, sign bit is at position 29)
            let is_negative = bytes[29] & 0x80 != 0;
            let mut tick_bytes = [0u8; 4];
            if is_negative {
                tick_bytes[0] = 0xFF; // Sign extend
            }
            tick_bytes[1..4].copy_from_slice(&bytes[29..32]);
            i32::from_be_bytes(tick_bytes)
        };
        let tick_upper = {
            let bytes: &[u8] = topics[3].as_ref();
            let is_negative = bytes[29] & 0x80 != 0;
            let mut tick_bytes = [0u8; 4];
            if is_negative {
                tick_bytes[0] = 0xFF;
            }
            tick_bytes[1..4].copy_from_slice(&bytes[29..32]);
            i32::from_be_bytes(tick_bytes)
        };
        let sender = Address::from_slice(&data_bytes[12..32]); // last 20 bytes
        let amount = u128::from_be_bytes(data_bytes[48..64].try_into()?); // last 16 bytes of second word
        Ok(PoolUpdate::Mint {
            tick_lower,
            tick_upper,
            amount,
        })
    }

    fn decode_burn_with_sol(log: &Log) -> anyhow::Result<PoolUpdate> {
        let data = log.data();
        let data_bytes: &[u8] = &data.data;
        if data_bytes.len() < 128 {
            return Err(anyhow::anyhow!("Insufficient data lenght"));
        }
        let topics = log.topics();
        let owner = Address::from_slice(&topics[1][12..]);
        let tick_lower = {
            let bytes: &[u8] = topics[2].as_ref();
            // Check sign bit (int24 is 3 bytes, sign bit is at position 29)
            let is_negative = bytes[29] & 0x80 != 0;
            let mut tick_bytes = [0u8; 4];
            if is_negative {
                tick_bytes[0] = 0xFF; // Sign extend
            }
            tick_bytes[1..4].copy_from_slice(&bytes[29..32]);
            i32::from_be_bytes(tick_bytes)
        };
        let tick_upper = {
            let bytes: &[u8] = topics[3].as_ref();
            let is_negative = bytes[29] & 0x80 != 0;
            let mut tick_bytes = [0u8; 4];
            if is_negative {
                tick_bytes[0] = 0xFF;
            }
            tick_bytes[1..4].copy_from_slice(&bytes[29..32]);
            i32::from_be_bytes(tick_bytes)
        };
        let amount = u128::from_be_bytes(data_bytes[16..32].try_into()?); // last 16 bytes of second word
        Ok(PoolUpdate::Burn {
            tick_lower,
            tick_upper,
            amount,
        })
    }
    // todo: unit tests here

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
        fee: u32,
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
