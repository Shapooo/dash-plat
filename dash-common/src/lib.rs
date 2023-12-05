use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct NewTransactionRequest {
    pub hash: [u8; 32],
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct TransactionReceipt {
    pub hash: [u8; 32],
    pub result: TransactionResult,
}

#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum TransactionResult {
    Commited,
    Unaccepted,
}
