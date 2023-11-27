use crate::{config, network, transaction::TransactionManager};

use anyhow::Result;

const PENDING_TRANSACTIONS: u64 = 100;
pub struct Client {
    pub network: network::Network,
    pub transaction_manager: TransactionManager,
}

impl Client {
    pub fn new(config: config::Config) -> Result<Self> {
        let quorum = config.node_addrs.len() as u64 / 3 + 1;
        let network = network::Network::new(config.node_addrs)?;
        Ok(Self {
            network,
            transaction_manager: TransactionManager::new(quorum),
        })
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            if self.transaction_manager.pending_sum() < PENDING_TRANSACTIONS {
                let transaction = self.transaction_manager.next()?;
                self.network.send_transaction(transaction)?;
            }

            if let Some(receipt) = self.network.receive_transaction_receipt()? {
                self.transaction_manager.collect_commit(receipt)?;
            }
        }
    }
}
