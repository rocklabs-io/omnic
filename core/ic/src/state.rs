use candid::{CandidType, Deserialize, Principal};
use std::collections::{HashSet, HashMap, BTreeMap};
use ic_web3::types::H256;
use std::iter::FromIterator;
use serde::Serialize;

use crate::types::{Message, MessageStable};

#[derive(CandidType, Deserialize, Copy, Clone, PartialEq, Eq)]
pub enum State {
    Init,
    Fetching(usize),
    End,
    Fail,
}

impl Default for State {
    fn default() -> Self {
        Self::Init
    }
}

/// state transition
/// chain ids: record all chain ids
/// rpc urls: record rpc urls for this round
/// block height: specific block height to query root
/// root: root
/// main state: loop forever to fetch root from each chain
/// sub state: each time issue an sub task to handle fetch root from specific rpc url
/// 
/// Init: inital state
/// Fetching(idx): fetching events
/// End: Round Finish
/// Fail: Fail to fetch or root mismatch
/// 
/// Main State transition:
/// Init => Move to Fetching(0)
/// Fetching(idx) => 
///     Sub state: Init => init rpc urls, chain_ids into state machine for this round, issue a sub task for fetch root of current chain id
///     Sub state: Fetching => fetching, do nothing
///     Sub state: Fail => Move sub state and state to Init
///     Sub state: End => Update the root, move sub state and state to Init
/// End, Fail => can't reach this 2 state in main state
/// 
/// Sub state transition:
///     Init => get block height, move state to fail or fetching, issue a sub task
///     Fetching => fetch root, compare and set the root, move state accordingly, issue a sub task
///     Fail => _
///     End => _
#[derive(CandidType, Deserialize, Default, Clone)]
pub struct StateMachine {
    pub chain_id: u32,
    pub rpc_urls: Vec<String>,
    pub omnic_addr: String,
    pub block_height: u64,
    pub cache_msg: HashMap<usize, Vec<MessageStable>>,
    pub state: State,
    pub sub_state: State
}

impl StateMachine {
    pub fn set_chain_id(&mut self, chain_id: u32) {
        self.chain_id = chain_id;
    }

    pub fn set_rpc_urls(&mut self, rpc_urls: Vec<String>) {
        self.rpc_urls = rpc_urls;
    }

    pub fn set_omnic_addr(&mut self, omnic_addr: String) {
        self.omnic_addr = omnic_addr;
    }


    pub fn rpc_count(&self) -> usize {
        self.rpc_urls.len()
    }

    pub fn get_fetching_next_state(&self) -> (State, State) {
        if let State::Fetching(_) = self.state {
            // state and sub state always move to init
            (State::Init, State::Init)
        } else {
            panic!("state not in fetching")
        }
    }

    pub fn get_fetching_next_sub_state(&self, check_result: bool) -> State {
        if let State::Fetching(idx) = self.sub_state {
            if!check_result {
                State::Fail
            } else if idx + 1 == self.rpc_count() {
                State::End
            } else {
                State::Fetching(idx + 1)
            }
        } else {
            panic!("sub state not in fetching")
        }
    }
}

// impl From<StateMachineStable> for StateMachine {
//     fn from(s: StateMachineStable) -> Self {
//         Self {
//             chain_id: s.chain_id,
//             rpc_urls: s.rpc_urls,
//             last_scanned_block: s.last_scanned_block,
//             omnic_addr: s.omnic_addr,
//             events: HashMap::from_iter(s.events.into_iter().map(|(x, y)| (H256::from(x), Message::from(y)))),
//             state: s.state,
//             sub_state: s.sub_state,
//         }
//     }
// }

// impl From<StateMachine> for StateMachineStable {
//     fn from(s: StateMachine) -> Self {
//         Self {
//             chain_id: s.chain_id,
//             rpc_urls: s.rpc_urls,
//             last_scanned_block: s.last_scanned_block,
//             omnic_addr: s.omnic_addr,
//             events: Vec::from_iter(s.events.into_iter().map(|(x, y)| (x.to_fixed_bytes(), MessageStable::from(y)))),
//             state: s.state,
//             sub_state: s.sub_state,
//         }
//     }
// }

#[derive(CandidType, Deserialize, Clone)]
pub struct StateInfo {
    owners: HashSet<Principal>,
    scan_event_period: u64,
    query_rpc_number: u64,

}

impl StateInfo {
    pub fn default() -> StateInfo {
        StateInfo {
            owners: HashSet::default(),
            scan_event_period: 1_000_000_000 * 10,
            query_rpc_number: 1,
        }
    }

    pub fn add_owner(&mut self, owner: Principal) {
        self.owners.insert(owner);
    }

    pub fn delete_owner(&mut self, owner: Principal) {
        self.owners.remove(&owner);
    }

    pub fn is_owner(&self, user: Principal) -> bool {
        self.owners.contains(&user)
    }

    pub fn set_scan_period(&mut self, period: u64) {
        self.scan_event_period = period;
    }

    pub fn set_rpc_number(&mut self, n: u64) {
        self.query_rpc_number = n
    }
}

#[derive(CandidType, Deserialize, Default)]
pub struct RecordDB {
    pub records: Vec<Record>,
    // index
    pub op_index: BTreeMap<String, Vec<usize>>,
    // nonce
    pub out_nonce: HashMap<u32, HashMap<Principal, u64>>, // [dst_chain][canister_app] => nonce
    pub in_nonce: HashMap<u32, HashMap<String, u64>>, // [src_chain][app] => nonce
}

#[derive(CandidType, Deserialize, Clone)]
pub struct Record {
    pub id: usize, 
    pub caller: Principal,
    pub timestamp: u64,
    pub operation: String,
    pub details: Vec<(String, DetailValue)>,
}

// refer to caps https://github.com/Psychedelic/cap/blob/main/common/src/transaction.rs
#[derive(CandidType, Serialize, Deserialize, Clone, PartialEq)]
pub enum DetailValue {
    True,
    False,
    U64(u64),
    I64(i64),
    Float(f64),
    Text(String),
    Principal(Principal),
    #[serde(with = "serde_bytes")]
    Slice(Vec<u8>),
    Vec(Vec<DetailValue>),
}

impl RecordDB {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_out_nonce(&self, dst_chain: &u32, canister: &Principal) -> u64 {
        self.out_nonce.get(dst_chain).map_or(0u64, |item| item.get(canister).map_or(0u64, |n| *n))
    }

    pub fn get_in_nonce(&self, dst_chain: &u32, sender: &str) -> u64 {
        self.in_nonce.get(dst_chain).map_or(0u64, |item| item.get(sender).map_or(0u64, |n| *n))
    }

    pub fn inc_out_nonce(&mut self, dst_chain: u32, canister: Principal) {
        self.out_nonce.entry(dst_chain).and_modify(|item|
            {item.entry(canister.clone()).and_modify(|n| *n += 1).or_insert(0u64);}
        ).or_insert(HashMap::from([(canister, 0u64),]));
    }

    pub fn inc_in_nonce(&mut self, dst_chain: u32, sender: String) {
        self.in_nonce.entry(dst_chain).and_modify(|item|
            {item.entry(sender.clone()).and_modify(|n| *n += 1).or_insert(0u64);}
        ).or_insert(HashMap::from([(sender, 0u64),]));
    }

    pub fn size(&self, op: Option<String>) -> usize {
        match op {
            Some(o) => {
                match self.op_index.get(&o) {
                    Some(i) => i.len(),
                    None => 0,
                }
            }
            None => {
                self.records.len()
            }
        }
    }

    pub fn append(&mut self, caller: Principal, ts: u64, op: String, details: Vec<(String, DetailValue)>) -> usize {
        let id = self.size(None);
        let record = Record{
            id,
            caller,
            timestamp: ts,
            operation: op.clone(),
            details,
        };
        self.records.push(record);
        // store the operation index
        self.op_index
            .entry(op)
            .and_modify(|v| v.push(id))
            .or_insert(vec![id]);
        id
    }

    pub fn load_by_id(&self, id: usize) -> Option<Record> {
        self.records.get(id).cloned()
    }

    // start: inclusive, end: exclusive
    pub fn load_by_id_range(&self, start: usize, mut end: usize) -> Vec<Record> {
        if start > end {
            panic!("Invalid range");
        }
        let len = self.size(None);
        if len == 0 {
            return Vec::default();
        }
        if end > len {
            end = len
        }
        self.records.get(start..end).expect("error load by range").to_vec().clone()
    }

    // op: operation, start: inclusive, end: exclusive
    pub fn load_by_opeation(&self, op: String, start: usize, mut end: usize) -> Vec<Record> {
        if start > end {
            panic!("Invalid range");
        }
        let default = Vec::default();
        let ops = self.op_index.get(&op).unwrap_or(default.as_ref());
        let len = op.len();
        if len == 0 {
            return Vec::default();
        }
        if end > len {
            end = len
        }

        let mut res: Vec<Record> = Vec::default();
        for id in &ops[start..end] {
            let record = self.records.get(id.to_owned()).expect("error load by id").clone();
            res.push(record);
        }

        res
    }
}