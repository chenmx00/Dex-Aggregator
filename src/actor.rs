use alloy::{
    consensus::transaction::PooledTransaction,
    primitives::{Address, U256},
    providers::{Provider, ProviderBuilder},
    rpc::client::{ClientBuilder, RpcClient},
};
use alloy_transport_ws::WsConnect;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
struct PoolState {}
enum PoolMessage {
    GetPoolState {
        pool: Address,
        send_to: mpsc::Sender<Option<PoolState>>,
    },
    SimulateSwap {
        token_in: u128,
        pool: Address,
        send_to: mpsc::Sender<Option<u128>>,
    },
    FindPool {
        token_0: Address,
        token_1: Address,
        fee: u128,
        send_to: mpsc::Sender<Option<PoolState>>,
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

    async fn on_message(&self, message: PoolMessage) {
        match message {
            PoolMessage::GetPoolState { pool, send_to } => { self.get_pool_state()}
        }
    }

    async fn simulate_swap() {}

    async fn get_pool_state() {}

    async fn run() {}
}

struct PoolActorHandler {}

impl PoolActorHandler {}
