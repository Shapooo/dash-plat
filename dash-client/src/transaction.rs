use dash_common::{NewTransactionRequest, TransactionHash, TransactionReceipt};

use std::collections::{hash_map::Entry, HashMap};

use anyhow::Result;
use chrono::{DateTime, Local};
use hotstuff_rs::types::PublicKeyBytes;
use log::error;
use rand::{thread_rng, Rng};
use sha2::{Digest, Sha256};

type TransactionTimestamp = (DateTime<Local>, DateTime<Local>);

#[derive(Clone, Debug)]
pub struct TransactionManager {
    quorum: u64,
    sequence_number: u64,
    pending_transactions: HashMap<TransactionHash, (DateTime<Local>, u64)>,
    commited_transactions: HashMap<TransactionHash, TransactionTimestamp>,
    pubkey: PublicKeyBytes,
}

impl TransactionManager {
    pub fn new(quorum: u64, pubkey: PublicKeyBytes) -> Self {
        Self {
            quorum,
            sequence_number: Default::default(),
            pending_transactions: Default::default(),
            commited_transactions: Default::default(),
            pubkey,
        }
    }

    pub fn next(&mut self) -> Result<NewTransactionRequest> {
        let data = generate_random_bytes(128);
        let hash: TransactionHash = Sha256::digest(&data).into();
        let transaction = NewTransactionRequest {
            requester: self.pubkey,
            hash,
            data,
        };
        self.sequence_number = self.sequence_number.wrapping_add(1);
        self.pending_transactions
            .insert(transaction.hash, (Local::now(), 0));
        Ok(transaction)
    }

    pub fn collect_commit(&mut self, receipt: TransactionReceipt) -> Result<()> {
        match self.pending_transactions.entry(receipt.hash) {
            Entry::Occupied(mut entry) => {
                let (_, commited_sum) = entry.get().clone();
                if commited_sum >= self.quorum {
                    let (start, _) = entry.remove();
                    self.commited_transactions
                        .insert(receipt.hash, (start, Local::now()));
                } else {
                    entry.get_mut().1 += 1;
                }
            }
            Entry::Vacant(_) => {
                error!("unknown transaction");
            }
        }
        Ok(())
    }

    pub fn pending_sum(&self) -> u64 {
        self.pending_transactions.len() as u64
    }
}

fn generate_random_bytes(length: usize) -> Vec<u8> {
    let mut rng = thread_rng();
    let mut result = vec![0; length];
    rng.fill(&mut result[..]);
    result
}
