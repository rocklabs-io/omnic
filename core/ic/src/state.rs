use candid::{CandidType, Deserialize, Principal};
use std::collections::{HashSet, HashMap};
use ic_web3::types::H256;
use std::iter::FromIterator;

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
/// Fetching(idx): fetching roots
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
#[derive(Default, Clone)]
pub struct StateMachine {
    pub chain_id: u32,
    pub rpc_urls: Vec<String>,
    pub block_height: u64,
    pub omnic_addr: String,
    pub roots: HashMap<H256, usize>,
    pub state: State,
    pub sub_state: State
}

#[derive(CandidType, Deserialize)]
pub struct StateMachineStable {
    chain_id: u32,
    rpc_urls: Vec<String>,
    block_height: u64,
    omnic_addr: String,
    roots: Vec<([u8;32], usize)>,
    state: State,
    sub_state: State
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

impl From<StateMachineStable> for StateMachine {
    fn from(s: StateMachineStable) -> Self {
        Self {
            chain_id: s.chain_id,
            rpc_urls: s.rpc_urls,
            block_height: s.block_height,
            omnic_addr: s.omnic_addr,
            roots: HashMap::from_iter(s.roots.into_iter().map(|(x, y)| (H256::from(x), y))),
            state: s.state,
            sub_state: s.sub_state,
        }
    }
}

impl From<StateMachine> for StateMachineStable {
    fn from(s: StateMachine) -> Self {
        Self {
            chain_id: s.chain_id,
            rpc_urls: s.rpc_urls,
            block_height: s.block_height,
            omnic_addr: s.omnic_addr,
            roots: Vec::from_iter(s.roots.into_iter().map(|(x, y)| (x.to_fixed_bytes(), y))),
            state: s.state,
            sub_state: s.sub_state,
        }
    }
}

#[derive(CandidType, Deserialize, Clone)]
pub struct StateInfo {
    pub owners: HashSet<Principal>,
    pub fetch_root_period: u64,
    pub fetch_roots_period: u64,
    pub query_rpc_number: u64,
}

impl StateInfo {
    pub fn default() -> StateInfo {
        StateInfo {
            owners: HashSet::default(),
            fetch_root_period: 1_000_000_000 * 30,
            fetch_roots_period: 1_000_000_000 * 60,
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

    pub fn set_fetch_period(&mut self, v1: u64, v2: u64) {
        self.fetch_root_period = v1;
        self.fetch_roots_period = v2;
    }

    pub fn set_rpc_number(&mut self, n: u64) {
        self.query_rpc_number = n
    }
}