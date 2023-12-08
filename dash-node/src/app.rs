use crate::kv_store;
use dash_common::NewTransactionRequest;

use hotstuff_rs::app;
use log::trace;
use tokio::sync::mpsc::Receiver;

pub struct AppImpl {
    block_rx: Receiver<NewTransactionRequest>,
}

impl AppImpl {
    pub fn new(block_rx: Receiver<NewTransactionRequest>) -> Self {
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
        let request = match self.block_rx.blocking_recv() {
            Some(request) => request,
            None => panic!("block channel disconnected"),
        };
        trace!("produce_block {:?}", request.hash);
        app::ProduceBlockResponse {
            data_hash: request.hash,
            data: vec![request.data],
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
