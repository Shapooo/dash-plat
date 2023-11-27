use crate::message::{NewTransactionRequest, TransactionReceipt};

use std::collections::{hash_map::Entry, HashMap};

use anyhow::Result;
use chrono::{DateTime, Local};
use log::error;
use rand::{thread_rng, Rng};

type TransactionTimestamp = (DateTime<Local>, DateTime<Local>);

#[derive(Clone, Debug)]
pub struct TransactionManager {
    quorum: u64,
    sequence_number: u64,
    pending_transactions: HashMap<u64, (DateTime<Local>, u64)>,
    commited_transactions: HashMap<u64, TransactionTimestamp>,
}

impl TransactionManager {
    pub fn new(quorum: u64) -> Self {
        Self {
            quorum,
            sequence_number: Default::default(),
            pending_transactions: Default::default(),
            commited_transactions: Default::default(),
        }
    }

    pub fn next(&mut self) -> Result<NewTransactionRequest> {
        let content = generate_random_bytes(128);
        let transaction = NewTransactionRequest {
            id: self.sequence_number,
            content,
        };
        self.sequence_number = self.sequence_number.wrapping_add(1);
        self.pending_transactions
            .insert(transaction.id, (Local::now(), 0));
        Ok(transaction)
    }

    pub fn collect_commit(&mut self, receipt: TransactionReceipt) -> Result<()> {
        match self.pending_transactions.entry(receipt.id) {
            Entry::Occupied(mut entry) => {
                let (_, commited_sum) = entry.get().clone();
                if commited_sum >= self.quorum {
                    let (start, _) = entry.remove();
                    self.commited_transactions
                        .insert(receipt.id, (start, Local::now()));
                } else {
                    entry.insert((Local::now(), commited_sum + 1));
                }
            }
            Entry::Vacant(_) => {
                error!("unknown transaction id: {}", receipt.id);
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
