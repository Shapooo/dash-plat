use borsh::{BorshDeserialize, BorshSerialize};
use hotstuff_rs::types::PublicKeyBytes;

pub type TransactionHash = [u8; 32];

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct NewTransactionRequest {
    pub requester: PublicKeyBytes,
    pub hash: TransactionHash,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct TransactionReceipt {
    pub receiptor: PublicKeyBytes,
    pub requester: PublicKeyBytes,
    pub hash: TransactionHash,
    pub result: TransactionResult,
}

#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum TransactionResult {
    Commited,
    Unaccepted,
}
