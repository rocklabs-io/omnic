/*
omnic proxy canister:
    fetch_root: fetch merkel roots from all supported chains and insert to chain state
*/

use std::cell::{RefCell};
use std::collections::{HashMap, VecDeque};
use std::convert::TryInto;
use std::iter::FromIterator;

use ic_cron::task_scheduler::TaskScheduler;
use ic_web3::types::H256;
use ic_web3::ic::get_eth_addr;

use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update, heartbeat};
use ic_cdk::export::candid::{candid_method, CandidType, Deserialize};
use ic_cdk::api::management_canister::http_request::HttpResponse;
use candid::types::principal::Principal;

use ic_cron::types::Iterations;

use accumulator::{TREE_DEPTH, merkle_root_from_branch};
use omnic::{Message, chains::EVMChainClient, ChainConfig, ChainState, ChainType};
use omnic::HomeContract;
use omnic::consts::{KEY_NAME, MAX_RESP_BYTES, CYCLES_PER_CALL, CYCLES_PER_BYTE};
use omnic::state::{State, StateMachine, StateMachineStable, StateInfo};
use omnic::call::{call_to_canister, call_to_chain};
use omnic::utils::check_roots_result;

ic_cron::implement_cron!();

#[derive(CandidType, Deserialize, Clone)]
enum Task {
    FetchRoots,
    FetchRoot
}

thread_local! {
    static STATE_INFO: RefCell<StateInfo> = RefCell::new(StateInfo::default());
    static CHAINS: RefCell<HashMap<u32, ChainState>>  = RefCell::new(HashMap::new());
    static STATE_MACHINE: RefCell<StateMachine> = RefCell::new(StateMachine::default());
    static LOGS: RefCell<VecDeque<String>> = RefCell::new(VecDeque::default());
}

#[query]
#[candid_method(query)]
fn get_logs() -> Vec<String> {
    LOGS.with(|l| {
        l.borrow().clone().into()
    })
}

fn get_fetch_root_period() -> u64 {
    STATE_INFO.with(|s| s.borrow().fetch_root_period)
}

fn get_fetch_roots_period() -> u64 {
    STATE_INFO.with(|s| s.borrow().fetch_roots_period)
}

#[init]
#[candid_method(init)]
fn init() {
    let caller = ic_cdk::api::caller();
    STATE_INFO.with(|info| {
        let mut info = info.borrow_mut();
        info.add_owner(caller);
    });

    // set up cron job
    cron_enqueue(
        Task::FetchRoots, 
        ic_cron::types::SchedulingOptions {
            delay_nano: get_fetch_roots_period(),
            interval_nano: get_fetch_roots_period(),
            iterations: Iterations::Infinite,
        },
    ).unwrap();
}

#[query]
async fn transform(raw: HttpResponse) -> HttpResponse {
    let mut t = raw;
    t.headers = vec![];
    t
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
                    add_log("chain type not supported yet!".to_string());
                }
            }
        }
    });
    Ok(true)
}

#[update(guard = "is_authorized")]
#[candid_method(update, rename = "set_fetch_period")]
async fn set_fetch_period(fetch_root_period: u64, fetch_roots_period: u64) -> Result<bool, String> {
    STATE_INFO.with(|s| {
        let mut s = s.borrow_mut();
        s.set_fetch_period(fetch_root_period, fetch_roots_period);
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

#[update(name = "delete_chain", guard = "is_authorized")]
#[candid_method(update, rename = "delete_chain")]
fn delete_chain(chain_id: u32) -> Result<bool, String> {
    CHAINS.with(|c| {
        let mut chains = c.borrow_mut();
        match chains.remove(&chain_id) {
            Some(_) => { Ok(true) }
            None => { Err("Chain id not exist".to_string()) }
        }
    })
}

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

// update chain settings
#[update(guard = "is_authorized")]
#[candid_method(update, rename = "set_next_index")]
fn set_next_index(
    chain_id: u32, 
    next_index: u32
) -> Result<bool, String> {
    // add chain config
    CHAINS.with(|chains| {
        let mut chains = chains.borrow_mut();
        let mut chain = chains.get_mut(&chain_id).expect("chain id not found!");
        chain.next_index = next_index;
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

#[update(name = "fetch_root")]
#[candid_method(update, rename = "fetch_root")]
async fn fetch(chain_id: u32, height: u64) -> Result<String, String> {
    let (_caller, omnic_addr, rpc) = CHAINS.with(|chains| {
        let chains = chains.borrow();
        let c = chains.get(&chain_id).expect("chain not found");
        (c.canister_addr.clone(), c.config.omnic_addr.clone(), c.config.rpc_urls[0].clone())
    });

    let client = EVMChainClient::new(rpc, omnic_addr, MAX_RESP_BYTES, CYCLES_PER_CALL)
        .map_err(|e| format!("init client failed: {:?}", e))?;
    client.get_latest_root(Some(height))
        .await
        .map(|v| hex::encode(v))
        .map_err(|e| format!("get root err: {:?}", e))
}

#[update(name = "get_tx_count")]
#[candid_method(update, rename = "get_tx_count")]
async fn get_tx_count(chain_id: u32, addr: String) -> Result<u64, String> {
    // check cycles
    let available = ic_cdk::api::call::msg_cycles_available();
    let need_cycles = 10u64 * CYCLES_PER_BYTE;
    if available < need_cycles {
        return Err(format!("Insufficient cycles: require {} cycles. Received {}.", need_cycles, available));
    }
    // accept cycles
    let _accepted = ic_cdk::api::call::msg_cycles_accept(need_cycles);

    // get tx count
    let (chain_type, rpc_url, omnic_addr) = CHAINS.with(|c| {
        let chains = c.borrow();
        let chain = chains.get(&chain_id).expect("src chain id not exist");
        (chain.chain_type(), chain.config.rpc_urls[0].clone(), chain.config.omnic_addr.clone())
    });
    match chain_type {
        ChainType::Evm => {},
        _ => return Err("chain type not supported yet".into()),
    }

    let client = EVMChainClient::new(rpc_url, omnic_addr, MAX_RESP_BYTES, CYCLES_PER_CALL)
        .map_err(|e| format!("init client failed: {:?}", e))?;

    client.get_tx_count(addr)
        .await
        .map_err(|e| format!("{:?}", e))
}

#[update(name = "get_gas_price")]
#[candid_method(update, rename = "get_gas_price")]
async fn get_gas_price(chain_id: u32) -> Result<u64, String> {
    // check cycles
    let available = ic_cdk::api::call::msg_cycles_available();
    let need_cycles = 10u64 * CYCLES_PER_BYTE;
    if available < need_cycles {
        return Err(format!("Insufficient cycles: require {} cycles. Received {}.", need_cycles, available));
    }
    // accept cycles
    let _accepted = ic_cdk::api::call::msg_cycles_accept(need_cycles);

    // get gas price
    let (chain_type, rpc_url, omnic_addr) = CHAINS.with(|c| {
        let chains = c.borrow();
        let chain = chains.get(&chain_id).expect("src chain id not exist");
        (chain.chain_type(), chain.config.rpc_urls[0].clone(), chain.config.omnic_addr.clone())
    });
    match chain_type {
        ChainType::Evm => {},
        _ => return Err("chain type not supported yet".into()),
    }

    let client = EVMChainClient::new(rpc_url, omnic_addr, MAX_RESP_BYTES, CYCLES_PER_CALL)
        .map_err(|e| format!("init client failed: {:?}", e))?;

    client.get_gas_price()
        .await
        .map_err(|e| format!("{:?}", e))
}

// relayer canister call this to check if a message is valid before process_message
#[query(name = "is_valid")]
#[candid_method(query, rename = "is_valid")]
fn is_valid(message: Vec<u8>, proof: Vec<Vec<u8>>, leaf_index: u32) -> Result<bool, String> {
    // verify message proof: use proof, message to calculate the merkle root, 
    // check if the merkle root exists in corresponding chain state
    let m = Message::from_raw(message.clone()).map_err(|e| {
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

#[query(name = "get_next_index")]
#[candid_method(query, rename = "get_next_index")]
fn get_next_index(chain_id: u32) -> Result<u32, String> {
    CHAINS.with(|c| {
        let chains = c.borrow();
        let chain = chains.get(&chain_id).ok_or("src chain id not exist".to_string())?;
        Ok(chain.next_index())
    })
}

// application canister call this method to send tx to destination chain
#[update(name = "send_raw_tx")]
#[candid_method(update, rename = "send_raw_tx")]
async fn send_raw_tx(dst_chain: u32, raw_tx: Vec<u8>) -> Result<Vec<u8>, String> {
    // check cycles
    let available = ic_cdk::api::call::msg_cycles_available();
    let need_cycles = raw_tx.len() as u64 * CYCLES_PER_BYTE;
    if available < need_cycles {
        return Err(format!("Insufficient cycles: require {} cycles. Received {}.", need_cycles, available));
    }
    // accept cycles
    let _accepted = ic_cdk::api::call::msg_cycles_accept(need_cycles);

    // send tx
    let (chain_type, rpc_url, omnic_addr) = CHAINS.with(|c| {
        let chains = c.borrow();
        let chain = chains.get(&dst_chain).expect("src chain id not exist");
        (chain.chain_type(), chain.config.rpc_urls[0].clone(), chain.config.omnic_addr.clone())
    });
    match chain_type {
        ChainType::Evm => {},
        _ => return Err("chain type not supported yet".into()),
    }

    let client = EVMChainClient::new(rpc_url, omnic_addr, MAX_RESP_BYTES, CYCLES_PER_CALL)
        .map_err(|e| format!("init client failed: {:?}", e))?;

    // client.send_raw_tx will always end up with error because the same tx will be submitted multiple times 
    // by the node in the subnet, first submission response ok, the rest will response error,
    // so we should ignore return value of send_raw_tx, then query by the txhash to make sure the tx is correctly sent
    client.send_raw_tx(raw_tx)
        .await
        .map_err(|e| format!("{:?}", e))
    // TODO: fetch via client.get_tx_by_hash to make sure the tx is included
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
        add_log("message does not pass verification!".to_string());
        return Err("message does not pass verification!".into());
    }
    let m = Message::from_raw(message.clone()).map_err(|e| {
        format!("parse message from bytes failed: {:?}", e)
    })?;
    // check leaf_index == next_index, then bump next_index
    let next_index = CHAINS.with(|chains| {
        let chains = chains.borrow();
        let c = chains.get(&m.origin).expect("chain not found");
        c.next_index()
    });
    if next_index != leaf_index {
        add_log(format!("next_index: {} != leaf_index: {}, ", next_index, leaf_index));
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
        add_log(format!("recipient: {:?}", Principal::to_text(&recipient)));
        call_to_canister(recipient, &m).await
    } else {
        // send tx to dst chain
        // call_to_chain(m.destination, message).await
        let (caller, omnic_addr, rpc) = CHAINS.with(|chains| {
            let chains = chains.borrow();
            let c = chains.get(&m.destination).expect("chain not found");
            (c.canister_addr.clone(), c.config.omnic_addr.clone(), c.config.rpc_urls[0].clone())
        });
        if caller == "" || omnic_addr == "" {
            return Err("caller address is empty".into());
        }
        call_to_chain(caller, omnic_addr, rpc, m.destination, message).await
    }
}

#[update(name = "add_owner", guard = "is_authorized")]
#[candid_method(update, rename = "add_owner")]
async fn add_owner(owner: Principal) {
    STATE_INFO.with(|s| {
        s.borrow_mut().add_owner(owner);
    });
}

#[update(name = "remove_owner", guard = "is_authorized")]
#[candid_method(update, rename = "remove_owner")]
async fn remove_owner(owner: Principal) {
    STATE_INFO.with(|s| {
        s.borrow_mut().delete_owner(owner);
    });
}

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
                                let mut state = s.borrow_mut();
                                state.block_height = h;
                                state.roots = HashMap::default(); // reset roots in this round
                            });
                            State::Fetching(0)
                        },
                        Err(e) => {
                            add_log(format!("init contract failed: {}", e));
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
                    add_log(format!("root from {:?}: {:?}", state.rpc_urls[idx], root));
                    match root {
                        Ok(r) => {
                            incr_state_root(r);
                        },
                        Err(e) => {
                            add_log(format!("query root failed: {}", e));
                            incr_state_root(H256::zero());
                        },
                    };
                    STATE_MACHINE.with(|s| {
                        let s = s.borrow();
                        let (check_result, _) = check_roots_result(&s.roots, s.rpc_count());
                        s.get_fetching_next_sub_state(check_result)
                    })
                },
                Err(e) => {
                    add_log(format!("init evm chain client failed: {}", e));
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
        s.borrow_mut().sub_state = next_state;
    });

    if next_state != State::End && next_state != State::Fail {
        cron_enqueue(
            Task::FetchRoot, 
            ic_cron::types::SchedulingOptions {
                delay_nano: get_fetch_root_period(),
                interval_nano: get_fetch_root_period(),
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
            // get chain ids
            let chain_ids = CHAINS.with(|c| {
                Vec::from_iter(c.borrow().keys().cloned())
            });
            STATE_MACHINE.with(|s| {
                let mut state = s.borrow_mut();
                if chain_ids.len() > 0 {
                    state.chain_ids = chain_ids;
                    state.state = State::Fetching(0);
                }
            });
        }
        State::Fetching(idx) => {
            match state.sub_state {
                State::Init => {
                    // update rpc urls
                    let chain_id = state.chain_ids[idx];
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
                    add_log(format!("fetching for chain {:?}...", chain_id));
                    cron_enqueue(
                        Task::FetchRoot, 
                        ic_cron::types::SchedulingOptions {
                            delay_nano: get_fetch_root_period(),
                            interval_nano: get_fetch_root_period(),
                            iterations: Iterations::Exact(1),
                        },
                    ).unwrap();
                }
                State::Fetching(_) => {},
                State::End => {
                    // update root
                    CHAINS.with(|c| {
                        let mut chain = c.borrow_mut();
                        let chain_state = chain.get_mut(&state.chain_ids[idx]).expect("chain id not exist");
                        let (check_result, root) = check_roots_result(&state.roots, state.rpc_count());
                        if check_result {
                            chain_state.insert_root(root);
                        } else {
                            add_log(format!("invalid roots: {:?}", state.roots))
                        }
                    });
                    // update state
                    STATE_MACHINE.with(|s| {
                        let mut state = s.borrow_mut();
                        (state.state, state.sub_state) = state.get_fetching_next_state();
                    });
                },
                State::Fail => {
                    // update state
                    STATE_MACHINE.with(|s| {
                        let mut state = s.borrow_mut();
                        (state.state, state.sub_state) = state.get_fetching_next_state();
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

#[pre_upgrade]
fn pre_upgrade() {
    let chains = CHAINS.with(|c| {
        c.replace(HashMap::default())
    });
    let state_info = STATE_INFO.with(|s| {
        s.replace(StateInfo::default())
    });
    let state_machine = STATE_MACHINE.with(|s| {
        s.replace(StateMachine::default())
    });
    ic_cdk::storage::stable_save((chains, state_info, StateMachineStable::from(state_machine), _take_cron_state())).expect("pre upgrade error");
}

#[post_upgrade]
fn post_upgrade() {
    let (chains, 
        state_info, 
        state_machine, 
        cron_state
    ): (HashMap<u32, ChainState>, 
        StateInfo, 
        StateMachineStable, 
        Option<TaskScheduler>
    ) = ic_cdk::storage::stable_restore().expect("post upgrade error");
    
    CHAINS.with(|c| {
        c.replace(chains);
    });
    STATE_INFO.with(|s| {
        s.replace(state_info);
    });
    STATE_MACHINE.with(|s| {
        s.replace(state_machine.into());
    });
    _put_cron_state(cron_state);
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

fn incr_state_root(root: H256) {
    STATE_MACHINE.with(|s| {
        let mut state = s.borrow_mut();
        state
            .roots
            .entry(root)
            .and_modify(|c| *c += 1)
            .or_insert(1);
    })
}

fn add_log(log: String) {
    LOGS.with(|l| {
        let mut logs = l.borrow_mut();
        if logs.len() == 1000 {
            logs.pop_front();
        } 
        logs.push_back(log);
    });
}


fn main() {
    candid::export_service!();
    std::print!("{}", __export_service());
}