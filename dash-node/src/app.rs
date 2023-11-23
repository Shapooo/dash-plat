use crate::kv_store;
use hotstuff_rs::app;

pub struct AppImpl();

impl AppImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for AppImpl {
    fn default() -> Self {
        Self()
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
        app::ProduceBlockResponse {
            data_hash: [0; 32],
            data: Vec::new(),
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