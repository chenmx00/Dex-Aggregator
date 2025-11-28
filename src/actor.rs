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
use tokio::sync::mpsc;
struct PoolState {}
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

    async fn run(&mut self) -> anyhow::Result<()> {
        println!("Start indexing pools");
        if let Err(e) = self.index_pools().await {
            eprintln!("Error encountered indexing pools: {:?}", e);
            return Err(e);
        }
        Ok(())
    }
}

struct PoolActorHandler {}

impl PoolActorHandler {}
