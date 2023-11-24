use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshDeserialize, BorshSerialize)]
pub struct BlockMsg {
    pub data: Vec<u8>,
}
