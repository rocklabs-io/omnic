/*
omnic proxy canister:
    fetch_root: fetch merkel roots from all supported chains and insert to chain state
*/

use std::cell::{RefCell};
use std::collections::HashMap;
use std::convert::TryInto;

use ic_web3::types::H256;
use ic_web3::ic::get_eth_addr;

use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update, heartbeat};
use ic_cdk::export::candid::{candid_method, CandidType, Deserialize};
use ic_cdk::api::call::{call, CallResult};
use ic_cdk::export::Principal;

use ic_cron::types::Iterations;

use accumulator::{TREE_DEPTH, merkle_root_from_branch};
use omnic::{Message, chains::EVMChainClient, ChainConfig, ChainState, ChainType};
use omnic::Decode;
use omnic::HomeContract;
use omnic::consts::KEY_NAME;

ic_cron::implement_cron!();

const MAX_RESP_BYTES: Option<u64> = Some(300);
const CYCLES_PER_CALL: Option<u64> = None;

const FETCH_ROOTS_PERIOID: u64 = 1_000_000_000 * 30; //60 * 5; // 5 min in nano second
const FETCH_ROOT_PERIOID: u64 = 1_000_000_000 * 5; //60; // 1 min in nano second

#[derive(CandidType, Deserialize, Clone)]
enum Task {
    FetchRoots,
    FetchRoot
}

#[derive(Deserialize, Clone, PartialEq, Eq)]
enum State {
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
///     Sub state: Init => init rpc urls into state machine for this round, issue a sub task for fetch root of current chain id
///     Sub state: Fetching => fetching, do nothing
///     Sub state: Fail => Move sub state to Init, move state to fetching((idx + 1) % chains_num)
///     Sub state: End => Update the root, move sub state to Init, move state to fetching((idx + 1) % chains_num)
/// End, Fail => can't reach this 2 state in main state
/// 
/// Sub state transition:
///     Init => get block height, move state to fail or fetching, issue a sub task
///     Fetching => fetch root, compare and set the root, move state accordingly, issue a sub task
///     Fail => _
///     End => _
#[derive(Default, Clone)]
struct StateMachine {
    chain_ids: Vec<u32>,
    rpc_urls: Vec<String>,
    block_height: u64,
    omnic_addr: String,
    root: H256,
    state: State,
    sub_state: State
}

impl StateMachine {

    pub fn add_chain(&mut self, chain_id: u32) {
        if !self.chain_exists(chain_id) {
            self.chain_ids.push(chain_id)
        }
    }

    pub fn chain_exists(&self, chain_id: u32) -> bool {
        self.chain_ids.contains(&chain_id)
    }

    pub fn rpc_count(&self) -> usize {
        self.rpc_urls.len()
    }
}

#[derive(CandidType, Clone)]
struct StateInfo {
    owner: Principal,
}

impl StateInfo {
    pub fn default() -> StateInfo {
        StateInfo {
            owner: Principal::management_canister(),
        }
    }

    pub fn set_owner(&mut self, owner: Principal) {
        self.owner = owner;
    }

    pub fn is_owner(&self, user: Principal) -> bool {
        self.owner == user
    }
}

thread_local! {
    static STATE_INFO: RefCell<StateInfo> = RefCell::new(StateInfo::default());
    static CHAINS: RefCell<HashMap<u32, ChainState>>  = RefCell::new(HashMap::new());
    static STATE_MACHINE: RefCell<StateMachine> = RefCell::new(StateMachine::default());
}

#[init]
#[candid_method(init)]
fn init() {
    let caller = ic_cdk::api::caller();
    STATE_INFO.with(|info| {
        let mut info = info.borrow_mut();
        info.set_owner(caller);
    });

    // TODO: should we move this to a separate function, call this after chains are added?
    // yvon: it's ok too. for now I just make main state not go to FETCHING when chains is empty
    // set up cron job
    cron_enqueue(
        Task::FetchRoots, 
        ic_cron::types::SchedulingOptions {
            delay_nano: FETCH_ROOTS_PERIOID,
            interval_nano: FETCH_ROOTS_PERIOID,
            iterations: Iterations::Infinite,
        },
    ).unwrap();
}

// get canister's evm address
#[update(name = "get_canister_addr")]
#[candid_method(update, rename = "get_canister_addr")]
async fn get_canister_addr(chain_type: ChainType) -> Result<String, String> {
    let cid = ic_cdk::id();
    let derivation_path = vec![cid.clone().as_slice().to_vec()];
    match chain_type {
        ChainType::Evm => match get_eth_addr(Some(cid), Some(derivation_path), KEY_NAME.to_string()).await {
                Ok(addr) => { Ok(hex::encode(addr)) },
                Err(e) => { Err(e) },
            },
        _ => Err("chain type not supported yet!".into()),
    }
}

#[update(guard = "is_authorized")]
#[candid_method(update, rename = "set_canister_addrs")]
async fn set_canister_addrs() -> Result<bool, String> {
    let cid = ic_cdk::id();
    let derivation_path = vec![cid.clone().as_slice().to_vec()];
    let evm_addr = get_eth_addr(Some(cid), Some(derivation_path), KEY_NAME.to_string())
        .await
        .map(|v| hex::encode(v))
        .map_err(|e| format!("calc evm address failed: {:?}", e))?;
    CHAINS.with(|chains| {
        let mut chains = chains.borrow_mut();
        for (_id, chain) in chains.iter_mut() {
            match chain.chain_type() {
                ChainType::Evm => chain.set_canister_addr(evm_addr.clone()),
                _ => {
                    ic_cdk::println!("chain type not supported yet!");
                }
            }
        }
    });
    Ok(true)
}

#[update(guard = "is_authorized")]
#[candid_method(update, rename = "add_chain")]
fn add_chain(
    chain_id: u32, 
    urls: Vec<String>, 
    omnic_addr: String, 
    start_block: u64
) -> Result<bool, String> {
    // add chain config
    CHAINS.with(|chains| {
        let mut chains = chains.borrow_mut();
        if !chains.contains_key(&chain_id) {
            chains.insert(chain_id, ChainState::new(
                ChainConfig::new(
                    ChainType::Evm,
                    chain_id,
                    urls,
                    omnic_addr.into(),
                    start_block,
                )
            ));
        }
    });
    // add chain_id to state machine
    STATE_MACHINE.with(|s| {
        let mut state_machine = s.borrow_mut();
        // append s.chain_ids;
        if !state_machine.chain_exists(chain_id) {
            state_machine.add_chain(chain_id);    
        }
    });
    Ok(true)
}

// TODO: delete chain, what if state_machine is in progress?
// #[update(name = "delete_chain", guard = "is_authorized")]
// #[candid_method(update, rename = "delete_chain")]
// fn delete_chain() -> Result<bool, String> {

// }

// update chain settings
#[update(guard = "is_authorized")]
#[candid_method(update, rename = "update_chain")]
fn update_chain(
    chain_id: u32, 
    urls: Vec<String>, 
    omnic_addr: String, 
    start_block: u64
) -> Result<bool, String> {
    // add chain config
    CHAINS.with(|chains| {
        let mut chains = chains.borrow_mut();
        if chains.contains_key(&chain_id) {
            chains.insert(chain_id, ChainState::new(
                ChainConfig::new(
                    ChainType::Evm,
                    chain_id,
                    urls,
                    omnic_addr.into(),
                    start_block,
                )
            ));
        }
    });
    Ok(true)
}

#[query(name = "get_chains")]
#[candid_method(query, rename = "get_chains")]
fn get_chains() -> Result<Vec<ChainState>, String> {
    // add chain config
    CHAINS.with(|chains| {
        let chains = chains.borrow();
        Ok(chains.clone().into_iter().map(|(_id, c)| c).collect())
    })
}

// relayer canister call this to check if a message is valid before process_message
#[query(name = "is_valid")]
#[candid_method(query, rename = "is_valid")]
fn is_valid(message: Vec<u8>, proof: Vec<Vec<u8>>, leaf_index: u32) -> Result<bool, String> {
    // verify message proof: use proof, message to calculate the merkle root, 
    // check if the merkle root exists in corresponding chain state
    let m = Message::read_from(&mut message.clone().as_slice()).map_err(|e| {
        format!("parse message from bytes failed: {:?}", e)
    })?;
    let h = m.to_leaf();
    let p_h256: Vec<H256> = proof.iter().map(|v| H256::from_slice(&v)).collect();
    let p: [H256; TREE_DEPTH] = p_h256.try_into().map_err(|e| format!("parse proof failed: {:?}", e))?;
    // calculate root with leaf hash & proof
    let root = merkle_root_from_branch(h, &p, TREE_DEPTH, leaf_index as usize);
    // do not add optimistic yet
    CHAINS.with(|c| {
        let chains = c.borrow();
        let chain = chains.get(&m.origin).ok_or("src chain id not exist".to_string())?;
        Ok(chain.is_root_exist(root))
    })
}

#[query(name = "get_latest_root")]
#[candid_method(query, rename = "get_latest_root")]
fn get_latest_root(chain_id: u32) -> Result<String, String> {
    CHAINS.with(|c| {
        let chains = c.borrow();
        let chain = chains.get(&chain_id).ok_or("src chain id not exist".to_string())?;
        Ok(format!("{:x}", chain.latest_root()))
    })
}

#[update(name = "process_message")]
#[candid_method(update, rename = "process_message")]
async fn process_message(message: Vec<u8>, proof: Vec<Vec<u8>>, leaf_index: u32) -> Result<bool, String> {
    // verify message proof: use proof, message to calculate the merkle root, 
    // check if the root exists in corresponding chain state
    // if valid, call dest canister.handleMessage or send tx to dest chain
    // if invalid, return error
    let valid = is_valid(message.clone(), proof, leaf_index)?;
    if !valid {
        ic_cdk::println!("message does not pass verification!");
        return Err("message does not pass verification!".into());
    }
    let m = Message::read_from(&mut message.clone().as_slice()).map_err(|e| {
        format!("parse message from bytes failed: {:?}", e)
    })?;
    // check leaf_index == next_index, then bump next_index
    let next_index = CHAINS.with(|chains| {
        let chains = chains.borrow();
        let c = chains.get(&m.origin).expect("chain not found");
        c.next_index()
    });
    if next_index != leaf_index {
        ic_cdk::println!("next_index: {} != leaf_index: {}, ", next_index, leaf_index);
        return Err(format!("next_index: {} != leaf_index: {}, ", next_index, leaf_index));
    }
    CHAINS.with(|chains| {
        let mut chains = chains.borrow_mut();
        let c = chains.get_mut(&m.origin).expect("chain not found");
        c.bump_index();
    });
    // send msg to destination
    if m.destination == 0 {
        // take last 10 bytes
        let recipient = Principal::from_slice(&m.recipient.as_bytes()[22..]);
        ic_cdk::println!("recipient: {:?}", Principal::to_text(&recipient));
        call_to_canister(recipient, &m).await
    } else {
        // send tx to dst chain
        call_to_chain(m.destination, message).await
    }
}

async fn call_to_canister(recipient: Principal, m: &Message) -> Result<bool, String> {
    // call ic recipient canister
    let ret: CallResult<(Result<bool, String>,)> = 
        call(recipient, "handle_message", (m.origin, m.nonce, m.sender.as_bytes(), m.body.clone(), )).await;
    match ret {
        Ok((res, )) => {
            match res {
                Ok(_) => {
                    ic_cdk::println!("handle_message success!");
                },
                Err(err) => {
                    ic_cdk::println!("handle_message failed: {:?}", err);
                }
            }
            // message delivered
            Ok(true)
        },
        Err((_code, msg)) => {
            ic_cdk::println!("call app canister failed: {:?}", (_code, msg.clone()));
            // message delivery failed
            Err(format!("call app canister failed: {:?}", (_code, msg)))
        }
    }
}

async fn call_to_chain(dst_chain: u32, msg_bytes: Vec<u8>) -> Result<bool, String> {
    let (caller, omnic_addr, rpc) = CHAINS.with(|chains| {
        let chains = chains.borrow();
        let c = chains.get(&dst_chain).expect("chain not found");
        (c.canister_addr.clone(), c.config.omnic_addr.clone(), c.config.rpc_urls[0].clone())
    });
    if caller == "" || omnic_addr == "" {
        return Err("caller address is empty".into());
    }
    let client = EVMChainClient::new(rpc.clone(), omnic_addr.clone(), MAX_RESP_BYTES, CYCLES_PER_CALL)
        .map_err(|e| format!("init EVMChainClient failed: {:?}", e))?;
    client
        .dispatch_message(caller, dst_chain, msg_bytes)
        .await
        .map(|txhash| {
            ic_cdk::println!("dispatch_message txhash: {:?}", hex::encode(txhash));
            true
        })
        .map_err(|e| format!("dispatch_message failed: {:?}", e))
}

// TODO: aggregate roots from multiple different rpc providers
async fn fetch_root() {
    // query omnic contract.getLatestRoot, 
    // fetch from multiple rpc providers and aggregrate results, should be exact match
    let state = STATE_MACHINE.with(|s| {
        s.borrow().clone()
    });
    
    let next_state = match state.sub_state {
        State::Init => {
            match EVMChainClient::new(state.rpc_urls[0].clone(), state.omnic_addr.clone(), MAX_RESP_BYTES, CYCLES_PER_CALL) {
                Ok(client) => { 
                    match client.get_block_number().await {
                        Ok(h) => {
                            STATE_MACHINE.with(|s| {
                                s.borrow_mut().block_height = h;
                            });
                            State::Fetching(0)
                        },
                        Err(e) => {
                            ic_cdk::println!("init contract failed: {}", e);
                            State::Fail
                        },
                    }
                },
                Err(_e) => { 
                    State::Fail
                },
            }
        },
        State::Fetching(idx) => {
            // query root in block height
            match EVMChainClient::new(state.rpc_urls[0].clone(), state.omnic_addr.clone(), MAX_RESP_BYTES, CYCLES_PER_CALL) {
                Ok(client) => {
                    let root = client.get_latest_root(Some(state.block_height)).await;
                    ic_cdk::println!("root from {:?}: {:?}", state.rpc_urls[idx], root);
                    match root {
                        Ok(r) => {
                            if idx == 0 {
                                STATE_MACHINE.with(|s| {
                                    s.borrow_mut().root = r;
                                });
                                if idx + 1 == state.rpc_count() {
                                    State::End
                                } else {
                                    State::Fetching(idx + 1)
                                }
                            } else {
                                // compare and set the result with root
                                // if result != state.root, convert to fail
                                STATE_MACHINE.with(|s| {
                                    let s = s.borrow();
                                    if s.root != r {
                                        State::Fail
                                    } else {
                                        if idx + 1 == state.rpc_count() {
                                            State::End
                                        } else {
                                            State::Fetching(idx + 1)
                                        }
                                    }
                                })
                            }
                        },
                        Err(e) => {
                            ic_cdk::println!("query root failed: {}", e);
                            State::Fail
                        },
                    }
                },
                Err(e) => {
                    ic_cdk::println!("init evm chain client failed: {}", e);
                    State::Fail
                }
            }
        },
        State::End | State::Fail => {
            return
        },
    };

    // update sub state
    STATE_MACHINE.with(|s| {
        s.borrow_mut().sub_state = next_state.clone();
    });

    if next_state != State::End && next_state != State::Fail {
        cron_enqueue(
            Task::FetchRoot, 
            ic_cron::types::SchedulingOptions {
                delay_nano: FETCH_ROOT_PERIOID,
                interval_nano: FETCH_ROOT_PERIOID,
                iterations: Iterations::Exact(1),
            },
        ).unwrap();
    }
}

// this is done in heart_beat
async fn fetch_roots() {
    let state = STATE_MACHINE.with(|s| {
        s.borrow().clone()
    });

    match state.state {
        State::Init => {
            STATE_MACHINE.with(|s| {
                let mut state = s.borrow_mut();
                if state.chain_ids.len() > 0 {
                    state.state = State::Fetching(0);
                }
            });
        }
        State::Fetching(idx) => {
            match state.sub_state {
                State::Init => {
                    // update rpc urls
                    let chain_id = state.chain_ids[idx as usize];
                    let (rpc_urls, omnic_addr) = CHAINS.with(|c| {
                        let cs = c.borrow();
                        let chain = cs.get(&chain_id).unwrap();
                        (chain.config.rpc_urls.clone(), chain.config.omnic_addr.clone())
                    });
                    STATE_MACHINE.with(|s| {
                        let mut state = s.borrow_mut();
                        state.rpc_urls = rpc_urls;
                        state.omnic_addr = omnic_addr;
                    });
                    ic_cdk::println!("fetching for chain {:?}...", chain_id);
                    cron_enqueue(
                        Task::FetchRoot, 
                        ic_cron::types::SchedulingOptions {
                            delay_nano: FETCH_ROOT_PERIOID,
                            interval_nano: FETCH_ROOT_PERIOID,
                            iterations: Iterations::Exact(1),
                        },
                    ).unwrap();
                }
                State::Fetching(_) => {},
                State::End => {
                    // update root
                    CHAINS.with(|c| {
                        let mut chain = c.borrow_mut();
                        let chain_state = chain.get_mut(&state.chain_ids[idx as usize]).expect("chain id not exist");
                        chain_state.insert_root(state.root);
                    });
                    // update state
                    STATE_MACHINE.with(|s| {
                        let mut state = s.borrow_mut();
                        state.sub_state = State::Init;
                        state.state = State::Fetching((idx + 1) % state.chain_ids.len())
                    });
                },
                State::Fail => {
                    // update state
                    STATE_MACHINE.with(|s| {
                        let mut state = s.borrow_mut();
                        state.sub_state = State::Init;
                        state.state = State::Fetching((idx + 1) % state.chain_ids.len())
                    });
                },
            }
        },
        _ => { panic!("can't reach here")},
    }
}

#[heartbeat]
fn heart_beat() {
    for task in cron_ready_tasks() {
        let kind = task.get_payload::<Task>().expect("Serialization error");
        match kind {
            Task::FetchRoots => {
                ic_cdk::spawn(fetch_roots());
            },
            Task::FetchRoot => {
                ic_cdk::spawn(fetch_root());
            },
        }
    }
}

/// get the unix timestamp in second
// fn get_time() -> u64 {
//     ic_cdk::api::time() / 1000000000
// }

fn is_authorized() -> Result<(), String> {
    let user = ic_cdk::api::caller();
    STATE_INFO.with(|info| {
        let info = info.borrow();
        if !info.is_owner(user) {
            Err("unauthorized!".into())
        } else {
            Ok(())
        }
    })
}

fn main() {
    candid::export_service!();
    std::print!("{}", __export_service());
}