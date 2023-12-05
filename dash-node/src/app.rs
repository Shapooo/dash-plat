use crate::kv_store;

// use std::sync::mpsc::Receiver;

use hotstuff_rs::app;
use tokio::sync::mpsc::Receiver;

pub struct AppImpl {
    block_rx: Receiver<Vec<u8>>,
}

impl AppImpl {
    pub fn new(block_rx: Receiver<Vec<u8>>) -> Self {
        Self { block_rx }
    }
}

impl app::App<kv_store::KVStoreImpl> for AppImpl {
    fn chain_id(&self) -> hotstuff_rs::types::ChainID {
        1
    }

    fn produce_block(
        &mut self,
        _request: app::ProduceBlockRequest<kv_store::KVStoreImpl>,
    ) -> app::ProduceBlockResponse {
        let data = match self.block_rx.blocking_recv() {
            Some(data) => data,
            None => panic!("block channel disconnected"),
        };
        app::ProduceBlockResponse {
            data_hash: [0; 32],
            data: vec![data],
            app_state_updates: None,
            validator_set_updates: None,
        }
    }

    fn validate_block(
        &mut self,
        _request: app::ValidateBlockRequest<kv_store::KVStoreImpl>,
    ) -> app::ValidateBlockResponse {
        app::ValidateBlockResponse::Valid {
            app_state_updates: None,
            validator_set_updates: None,
        }
    }
}
