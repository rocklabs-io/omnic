
/*
    chain struct
    maintain a incoming message queue & a outgoing message queue
*/

use std::collections::{HashMap, VecDeque};
use crate::chain_config::ChainConfig;
use crate::Message;

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct Chain {
    pub config: ChainConfig,
    // TODO what's Byte32?   [u8; 32]?
    pub roots: HashMap<Bytes32, u64>, // root hash -> confirm time
    pub incoming_msgs: VecDeque<Message>,
    pub outgoing_msgs: VecDeque<Message>,
}

impl Chain {
    fn enqueue_incoming(&mut self, msg: Message) {
        self.incoming_msgs.push_back(msg);
    }

    fn enqueue_outgoing(&mut self, msg: Message) {
        self.outgoing_msgs.push_back(msg);
    }

    // TODO get messages from each queue, batch or one-by-one?

    // fn put_root()
}