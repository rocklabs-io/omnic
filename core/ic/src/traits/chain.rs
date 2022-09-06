
use ic_web3::types::H256;
use async_trait::async_trait;

use crate::types::{Message, RawMessage};
use crate::error::OmnicError;


#[async_trait]
pub trait HomeIndexer {
    async fn get_block_number(&self) -> Result<u32, OmnicError>;

    /// Fetch list of messages between blocks `from` and `to`.
    async fn fetch_sorted_messages(&self, _from: u32, _to: u32) -> Result<Vec<RawMessage>, OmnicError>;
}


#[async_trait]
pub trait HomeContract {
    async fn send_message(&self, msg: &Message) -> Result<Option<H256>, OmnicError>;
    async fn get_latest_root(&self, height: Option<u32>) -> Result<H256, OmnicError>;
}