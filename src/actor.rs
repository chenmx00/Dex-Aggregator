use alloy::{
    consensus::transaction::PooledTransaction,
    primitives::{Address, U256},
    providers::{Provider, ProviderBuilder},
    rpc::client::{ClientBuilder, RpcClient},
};
use alloy_transport_ws::WsConnect;
use anyhow::{self, Ok};
use std::sync::Arc;
use std::{collections::HashMap, ops::Add};
use tokio::sync::{RwLock, mpsc};
#[derive(Debug, Clone)]
struct PoolState {
    a: f32,
    b: f32,
}
enum PoolMessage {
    GetPoolState {
        pool: Address,
        send_to: mpsc::Sender<Option<PoolState>>,
    },
    SimulateSwap {
        token_in: U256,
        pool: Address,
        send_to: mpsc::Sender<Option<U256>>,
    },
    FindPool {
        token_0: Address,
        token_1: Address,
        fee: U256,
        send_to: mpsc::Sender<Option<Address>>,
    },
}
struct PoolActor {
    pools: HashMap<Address, PoolState>,
    provider: Arc<dyn Provider>,
    receiver: mpsc::Receiver<PoolMessage>,
    factory_address: Address,
}

impl PoolActor {
    fn new(
        provider: Arc<dyn Provider>,
        factory_address_url: String,
        receiver: mpsc::Receiver<PoolMessage>,
    ) -> Self {
        PoolActor {
            pools: HashMap::new(),
            provider,
            receiver,
            factory_address: factory_address_url.parse::<Address>().unwrap(),
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
                token_in,
                pool,
                send_to,
            } => {
                let token_out = self.simulate_swap(token_in, &pool);
                let _ = send_to.send(token_out);
                Ok(())
            }
        }
    }

    fn simulate_swap(&self, token_in: U256, pool: &Address) -> Option<U256> {
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

    async fn stream_in_rpc(
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
        let task_handle = tokio::spawn(async move {
            if let Err(e) = Self::stream_in_rpc(pools_for_stream, provider_for_stream).await {
                eprintln!("Error: {:?}", e);
            }
        });

        while let Some(msg) = self.receiver.recv().await {
            self.on_message(msg).await?;
        }
        Ok(())
    }
}

struct PoolActorHandler {}

impl PoolActorHandler {}
