use alloy::{
    consensus::transaction::PooledTransaction,
    primitives::{Address, U256},
    providers::{Provider, ProviderBuilder},
    rpc::client::{ClientBuilder, RpcClient},
};
use alloy_transport_ws::WsConnect;
use anyhow::anyhow;
use std::sync::Arc;
use std::{collections::HashMap, ops::Add};
use tokio::sync::{RwLock, mpsc, oneshot};

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
        fee: U256,
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

    fn find_pool(&self, token_0: &Address, token_1: &Address, fee: U256) -> Option<Address> {
        todo!();
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
        todo!();
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
