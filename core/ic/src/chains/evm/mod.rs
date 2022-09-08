
use ic_web3::types::{H256, H160, U64, BlockNumber, FilterBuilder};
use ic_web3::transports::ICHttp;
use ic_web3::Web3;
use std::convert::TryInto;
use std::str::FromStr;

use crate::traits::HomeIndexer;
use crate::types::RawMessage;
use crate::error::OmnicError;

use crate::chains::config::IndexerConfig;

const EVENT_SEND_MSG: &str = "84ec73a8411e8551ef1faab6c2277072efce9d5e4cc2ae5a218520dcdd7a377c";

#[derive(Clone)]
pub struct EVMChainIndexer {
    pub config: IndexerConfig,
    pub w3: Web3<ICHttp>,
}

impl EVMChainIndexer {
    pub fn new(
        config: IndexerConfig
    ) -> Result<Self, OmnicError> {
        Ok(EVMChainIndexer {
            config: config.clone(),
            w3: { 
                let http = ICHttp::new(&config.rpc_url, None, None)?; 
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
            .address(vec![H160::from_str(&self.config.omnic_addr).unwrap()])
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