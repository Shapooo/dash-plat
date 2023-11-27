use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct NewTransactionRequest {
    pub id: u64,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct TransactionReceipt {
    pub id: u64,
    pub result: TransactionResult,
}

#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum TransactionResult {
    Commited,
    Unaccepted,
}
