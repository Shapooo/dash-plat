use crate::kv_store::KVStoreImpl;
use dash_common::{NewTransactionRequest, TransactionHash};

use std::collections::{HashSet, VecDeque};

use hotstuff_rs::app::{
    App, ProduceBlockRequest, ProduceBlockResponse, ValidateBlockRequest, ValidateBlockResponse,
};
use log::trace;
use tokio::sync::mpsc::Receiver;

pub struct AppImpl {
    block_rx: Receiver<NewTransactionRequest>,
    trans_cache: VecDeque<NewTransactionRequest>,
    committed_block: HashSet<TransactionHash>,
    highest_committed_height: u64,
}

impl AppImpl {
    pub fn new(block_rx: Receiver<NewTransactionRequest>) -> Self {
        Self {
            block_rx,
            trans_cache: Default::default(),
            committed_block: Default::default(),
            highest_committed_height: 0,
        }
    }
}

impl App<KVStoreImpl> for AppImpl {
    fn chain_id(&self) -> hotstuff_rs::types::ChainID {
        1
    }

    fn produce_block(&mut self, request: ProduceBlockRequest<KVStoreImpl>) -> ProduceBlockResponse {
        loop {
            while let Ok(request) = self.block_rx.try_recv() {
                self.trans_cache.push_back(request);
            }
            let tree = request.block_tree();
            let mut pending_ancient = HashSet::new();
            if let Some(parent) = request.parent_block() {
                let hash = tree.block_data_hash(&parent).unwrap();
                pending_ancient.insert(hash);
                if let Some(grandparent) = tree.block_justify(&parent).and_then(|qc| {
                    if qc.is_genesis_qc() {
                        None
                    } else {
                        Some(qc.block)
                    }
                }) {
                    let hash = tree.block_data_hash(&grandparent).unwrap();
                    pending_ancient.insert(hash);
                    if let Some(great_grandparent) =
                        tree.block_justify(&grandparent).and_then(|qc| {
                            if qc.is_genesis_qc() {
                                None
                            } else {
                                Some(qc.block)
                            }
                        })
                    {
                        let hash = tree.block_data_hash(&great_grandparent).unwrap();
                        pending_ancient.insert(hash);
                    }
                }
                let parent_height = tree.block_height(&parent).unwrap();
                let highest_committed_height = parent_height.saturating_sub(3);
                for height in self.highest_committed_height + 1..=highest_committed_height {
                    let block = tree.block_at_height(height).unwrap();
                    let hash = tree.block_data_hash(&block).unwrap();
                    self.committed_block.insert(hash);
                }
            }
            while let Some(request) = self.trans_cache.pop_front() {
                trace!("produce_block");
                if pending_ancient.contains(&request.hash)
                    || self.committed_block.contains(&request.hash)
                {
                    continue;
                }
                return ProduceBlockResponse {
                    data_hash: request.hash,
                    data: vec![request.data],
                    app_state_updates: None,
                    validator_set_updates: None,
                };
            }
        }
    }

    fn validate_block(
        &mut self,
        request: ValidateBlockRequest<KVStoreImpl>,
    ) -> ValidateBlockResponse {
        if request
            .block_tree()
            .block_data_hash(&request.proposed_block().data_hash)
            .is_some()
        {
            ValidateBlockResponse::Invalid
        } else {
            ValidateBlockResponse::Valid {
                app_state_updates: None,
                validator_set_updates: None,
            }
        }
    }
}
