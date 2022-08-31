
/*
    used in relayer
    fetch crosschain messages from chain, maintain a merkle tree for the corresponding chain messages
    generate merkle proof and send message with proof to omnic proxy canister to process the message
*/

use std::collections::{HashMap, VecDeque};
use std::str::FromStr;
use ic_web3::transports::ICHttp;
use ic_web3::Web3;
use ic_web3::ic::{get_eth_addr, KeyInfo};
use ic_web3::{
    contract::{Contract, Options},
    ethabi::ethereum_types::{U64, U256, H256, H160},
    ethabi::{Event, EventParam, ParamType, Log as ABILog, RawLog},
    types::{Bytes, Address, TransactionParameters, BlockId, BlockNumber, FilterBuilder, Log},
};
use ic_cdk::export::candid::{CandidType, Deserialize};

use accumulator::{tree::Tree, Proof, Merkle};

use crate::chain_config::ChainConfig;
use crate::Message;

const EVENT_SEND_MSG: &str = "b9bede5465bf01e11c8b770ae40cbae2a14ace602a176c8ea626c9fb38a90bd8";
const EVENT_PROCESS_MSG: &str = "b9bede5465bf01e11c8b770ae40cbae2a14ace602a176c8ea626c9fb38a90bd8";

#[derive(Debug)]
pub struct ChainInfo {
    pub config: ChainConfig,
    pub tree: Tree<32>,
    pub incoming: VecDeque<Message>, // incoming messages
    pub confirming: VecDeque<Message>, // processed messages, wait confirmation
}

impl ChainInfo {
    pub fn new(config: ChainConfig) -> ChainInfo {
        ChainInfo {
            config: config,
            tree: Tree::<32>::default(),
            incoming: VecDeque::new(),
            confirming: VecDeque::new()
        }
    }

    pub fn set_current_block(&mut self, v: u64) {
        self.config.set_current_block(v);
    }

    pub fn set_batch_size(&mut self, v: u64) {
        self.config.set_batch_size(v);
    }

    pub async fn fetch_logs(&mut self) -> Result<(Vec<Log>, Vec<Log>), String> {
        let http = ICHttp::new(&self.config.rpc_urls[0], None, None).map_err(|e| format!("init ic http client failed: {:?}", e))?;
        let w3 = Web3::new(http);

        let event_send = H256::from_str(EVENT_SEND_MSG).unwrap();
        let event_proc = H256::from_str(EVENT_PROCESS_MSG).unwrap();
        let block_height: u64 = w3
            .eth().block_number().await
            .map(|h| h.as_u64())
            .map_err(|e| format!("get block height err: {:?}", e))?;
        let to_block = if self.config.current_block + self.config.batch_size < block_height {
            self.config.current_block + self.config.batch_size
        } else {
            block_height
        };
        let filter = FilterBuilder::default()
            .address(vec![H160::from_str(&self.config.omnic_addr).unwrap()])
            .topics(
                Some(vec![event_send, event_proc]),
                None,
                None,
                None,
            )
            .from_block(BlockNumber::Number(self.config.current_block.into()))
            .to_block(BlockNumber::Number(U64::from(to_block)))
            .build();
        let logs = w3.eth().logs(filter).await.map_err(|e| format!("get logs failed for chain: {:?}, {:?}", self.config, e))?;
        // update chainconfig.current_block
        self.config.set_current_block(to_block);
        // separate send event and process event
        let mut send_msgs = Vec::new();
        let mut proc_msgs = Vec::new();
        for log in logs {
            if log.topics[0] == event_send {
                send_msgs.push(log);
            } else if log.topics[0] == event_proc {
                proc_msgs.push(log);
            }
        }
        Ok((send_msgs, proc_msgs))
    }

    // generate merkle proof for given message
    pub fn generate_proof(&self, msg: Message) -> Proof<32> {
        self.tree.prove(msg.leaf_index.as_u32() as usize).unwrap()
    }

    // process SendMessage event logs
    pub fn process_send_logs(&mut self, logs: Vec<Log>) {
        // insert into tree & incoming msg queue
        for log in logs {
            let msg = if let Ok(v) = Message::from_log(&log) { v } else {
                // TODO: what if fails?
                continue;
            };
            self.incoming.push_back(msg.clone());
            self.tree.ingest(H256::from_slice(&msg.hash)); // could fail too
        }
    }

    // process ProcessMessage event logs
    pub fn process_proc_logs(&mut self, logs: Vec<Log>) {
        // remove the corresponding message from confirming queue
    }

    // check msgs in incoming queue, see if msgs are valid, 
    // if valid now, call proxyCanister.process_message with generated merkle proof and message body
    // otherwise, wait until valid
    pub fn process_msgs(&mut self, proxy_canister_root: H256, root_confirm_at: u64) {

    }
}