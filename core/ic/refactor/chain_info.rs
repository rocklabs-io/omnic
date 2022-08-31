
/*
    used in relayer
    fetch crosschain messages from chain, maintain a merkle tree for the corresponding chain messages
    generate merkle proof and send message with proof to omnic proxy canister to process the message
*/

use std::collections::{HashMap, VecDeque};
use crate::chain_config::ChainConfig;
use crate::Message;
use crate::accumulator::tree;

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct ChainInfo {
    pub config: ChainConfig,
    pub tree: tree::Tree<32>,
    pub incoming: VecDeque<Message>, // incoming messages
    pub confirming: VecDeque<Message>, // processed messages, wait confirmation
}

impl ChainInfo {
    pub fn set_current_block(&mut self, v: u64) {
        self.config.set_current_block(v);
    }

    pub fn set_batch_size(&mut self, v: u64) {
        self.config.set_batch_size(v);
    }

    pub fn generate_proof(&self, msg: Message) -> Proof<32> {

    }

    // pub async fn fetch_logs()

    pub fn process_logs(&mut self, logs: Vec<Log>) {
        for log in logs {
            // TODO: check if the log is SendMessage or ProcessMessage,
            // if SendMessage: insert into tree & incoming msg queue
            // if ProcessMessage: remove the corresponding message from confirming queue
            let msg = if let Ok(v) = Message::from_log(&log) { v } else {
                // TODO: what if fails?
                continue;
            };
            self.incoming.push_back(msg);
        }
    }
}