
/*
    chain struct
    maintain a incoming message queue & a outgoing message queue
*/

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct Chain {
    pub config: ChainConfig,
    pub roots: HashMap<Bytes32, u64>, // root hash -> confirm time
    pub incoming_msgs: Vec<Message>,
    pub outgoing_msgs: Vec<Message>,
}

impl Chain {

}