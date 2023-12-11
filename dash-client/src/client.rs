use crate::{config, network, transaction::TransactionManager};
use dash_common::crypto::publickey_to_base64;

use anyhow::Result;
use log::trace;

const PENDING_TRANSACTIONS: u64 = 10;
pub struct Client {
    pub network: network::Network,
    pub transaction_manager: TransactionManager,
}

impl Client {
    pub fn new(config: config::Config) -> Result<Self> {
        trace!("new client with config: {:?}", config);
        let quorum = config.node_addrs.len() as u64 / 3 * 2 + 1;
        let network = network::Network::new(config.node_addrs)?;
        let keypair = config.keypair.unwrap();
        let pubkey = keypair.public.to_bytes();
        Ok(Self {
            network,
            transaction_manager: TransactionManager::new(quorum, pubkey),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            if self.transaction_manager.pending_sum() < PENDING_TRANSACTIONS {
                trace!(
                    "pending transaction: {}, so send new transaction",
                    self.transaction_manager.pending_sum()
                );
                let transaction = self.transaction_manager.generate_transaction()?;
                self.network.send_transaction(transaction).await?;
            }

            if let Some(receipt) = self.network.receive_transaction_receipt().await? {
                trace!(
                    "recvd receipt from: {}",
                    publickey_to_base64(receipt.receiptor)
                );
                self.transaction_manager.collect_commit(receipt)?;
            }
        }
    }
}
