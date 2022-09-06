
use ic_web3::types::{H256, H160, U64, BlockNumber, FilterBuilder};
use ic_web3::transports::ICHttp;
use ic_web3::Web3;
use std::convert::TryInto;
use std::str::FromStr;

use crate::traits::HomeIndexer;
use crate::types::RawMessage;
use crate::error::OmnicError;

const EVENT_SEND_MSG: &str = "b9bede5465bf01e11c8b770ae40cbae2a14ace602a176c8ea626c9fb38a90bd8";

pub struct EVMChainIndexer {
    pub chain_id: u32,
    pub rpc_url: String,
    pub omnic_addr: String,
    pub start_block: u32,
    pub w3: Web3<ICHttp>,
}

impl EVMChainIndexer {
    pub fn new(
        chain_id: u32,
        rpc_url: String,
        omnic_addr: String,
        start_block: u32
    ) -> Result<Self, OmnicError> {
        Ok(EVMChainIndexer {
            chain_id,
            rpc_url: rpc_url.clone(),
            omnic_addr,
            start_block,
            w3: { 
                let http = ICHttp::new(&rpc_url, None, None)?; 
                Web3::new(http)
            },
        })
    }
}

#[async_trait::async_trait]
impl HomeIndexer for EVMChainIndexer {
    async fn get_block_number(&self) -> Result<u32, OmnicError> {
        let block_height: u32 = self.w3.eth().block_number().await.map(|h| h.as_u32())?;
        Ok(block_height)
    }

    /// fetch messages between blocks `from` and `to`.
    async fn fetch_sorted_messages(&self, _from: u32, _to: u32) -> Result<Vec<RawMessage>, OmnicError> {
        // TODO: take care of unwraps in this function
        let event_send = H256::from_str(EVENT_SEND_MSG).unwrap();
        
        let filter = FilterBuilder::default()
            .address(vec![H160::from_str(&self.omnic_addr).unwrap()])
            .topics(
                Some(vec![event_send]),
                None,
                None,
                None,
            )
            .from_block(BlockNumber::Number(U64::from(_from)))
            .to_block(BlockNumber::Number(U64::from(_to)))
            .build();
        let logs = self.w3.eth().logs(filter).await?;
        let mut msgs: Vec<RawMessage> = logs.iter().map(|log| {
            log.clone().try_into().unwrap()
        }).collect();
        msgs.sort_by(|a, b| {
            a.leaf_index.cmp(&b.leaf_index)
        });
        Ok(msgs)
    }
}

// impl Home for EVMChainHome {
//     async fn get_latest_root(&self, height: Option<u32>) -> Result<H256>;
//     async fn send_message(&self, msg: &Message) -> Result<Option<H256>>;
// }