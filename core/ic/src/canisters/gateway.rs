/*
omnic proxy canister:
    fetch_root: fetch merkel roots from all supported chains and insert to chain state
*/

use std::cell::{RefCell};
use std::collections::{HashMap, VecDeque};
use std::convert::TryInto;

use rand::{rngs::StdRng, SeedableRng};
use rand::seq::SliceRandom;

use ic_cdk::api::management_canister::http_request::{HttpResponse, TransformArgs};
use ic_cron::task_scheduler::TaskScheduler;
use ic_web3::types::H256;

use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update, heartbeat};
use ic_cdk::export::candid::{candid_method, CandidType, Deserialize};
use candid::types::principal::Principal;

use ic_cron::types::Iterations;

use accumulator::{TREE_DEPTH, merkle_root_from_branch};
use omnic::{Message, chains::EVMChainClient, ChainConfig, ChainState, ChainType};
use omnic::HomeContract;
use omnic::consts::{MAX_RESP_BYTES, CYCLES_PER_CALL, CYCLES_PER_BYTE};
use omnic::state::{State, StateMachine, StateInfo};
use omnic::utils::check_scan_message_results;

ic_cron::implement_cron!();

#[derive(CandidType, Deserialize, Clone)]
enum Task {
    Scan
}

thread_local! {
    static STATE_INFO: RefCell<StateInfo> = RefCell::new(StateInfo::default());
    static CHAINS: RefCell<ChainState>  = RefCell::new(ChainState::default());
    static STATE_MACHINE: RefCell<StateMachine> = RefCell::new(StateMachine::default());
    static LOGS: RefCell<VecDeque<String>> = RefCell::new(VecDeque::default());
}

#[query]
async fn transform(raw: TransformArgs) -> HttpResponse {
    let mut t = raw.response;
    t.headers = vec![];
    t
}

#[query]
#[candid_method(query)]
fn get_logs() -> Vec<String> {
    LOGS.with(|l| {
        l.borrow().clone().into()
    })
}

fn get_scan_event_period() -> u64 {
    STATE_INFO.with(|s| s.borrow().scan_event_period)
}

fn get_query_rpc_number() -> u64 {
    STATE_INFO.with(|s| s.borrow().query_rpc_number)
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
        Task::Scan, 
        ic_cron::types::SchedulingOptions {
            delay_nano: get_scan_event_period(),
            interval_nano: get_scan_event_period(),
            iterations: Iterations::Infinite,
        },
    ).unwrap();
}

#[update(guard = "is_authorized")]
#[candid_method(update, rename = "set_scan_event_period")]
async fn set_scan_event_period(scan_event_period: u64) -> Result<bool, String> {
    STATE_INFO.with(|s| {
        let mut s = s.borrow_mut();
        s.set_scan_period(scan_event_period);
    });
    Ok(true)
}

#[update(guard = "is_authorized")]
#[candid_method(update, rename = "set_rpc_number")]
async fn set_rpc_number(query_rpc_number: u64) -> Result<bool, String> {
    let rpc_url_count = CHAINS.with(|c| {
        let chain = c.borrow();
        chain.config.rpc_urls.len()
    });
    if query_rpc_number <= 0 || query_rpc_number > rpc_url_count as u64 {
        return Err("Invalid rpc number".to_string());
    }
    STATE_INFO.with(|s| {
        let mut s = s.borrow_mut();
        s.set_rpc_number(query_rpc_number);
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
    // set chain config
    CHAINS.with(|c| {
        c.replace(ChainState::new(
            ChainConfig::new(
                ChainType::Evm,
                chain_id,
                urls.clone(),
                ic_cdk::id(),
                omnic_addr.clone(),
                start_block,
            )
        ));
    });
    STATE_MACHINE.with(|s| {
        let mut state_machine = s.borrow_mut();
        state_machine.set_chain_id(chain_id);
        state_machine.set_rpc_urls(urls.clone());
        state_machine.set_omnic_addr(omnic_addr.clone());
    });
    Ok(true)
}

#[update(guard = "is_authorized")]
#[candid_method(update, rename = "add_urls")]
fn add_urls(
    urls: Vec<String>
) -> Result<bool, String> {
    // set chain config
    CHAINS.with(|c| {
        let mut c = c.borrow_mut();
        c.add_urls(urls);
    });
    Ok(true)
}

#[query(name = "get_chain")]
#[candid_method(query, rename = "get_chain")]
fn get_chain() -> Result<ChainState, String> {
    CHAINS.with(|chain| {
        let chain = chain.borrow();
        Ok(chain.clone())
    })
}

#[query(name = "get_state_info", guard = "is_authorized")]
#[candid_method(query, rename = "get_state_info")]
fn get_state_info() -> Result<StateInfo, String> {
    STATE_INFO.with(|info| {
        let info = info.borrow();
        Ok(info.clone())
    })
}

// #[update(name = "fetch_root")]
// #[candid_method(update, rename = "fetch_root")]
// async fn fetch(height: u64) -> Result<String, String> {
//     let (_, omnic_addr, rpc) = CHAINS.with(|chain| {
//         let chain = chain.borrow();
//         (chain.canister_addr.clone(), chain.config.omnic_addr.clone(), chain.config.rpc_urls[0].clone())
//     });

//     let client = EVMChainClient::new(rpc, omnic_addr, MAX_RESP_BYTES, CYCLES_PER_CALL)
//         .map_err(|e| format!("init client failed: {:?}", e))?;
//     client.get_latest_root(Some(height))
//         .await
//         .map(|v| hex::encode(v))
//         .map_err(|e| format!("get root err: {:?}", e))
// }

#[update(name = "get_tx_count")]
#[candid_method(update, rename = "get_tx_count")]
async fn get_tx_count(addr: String) -> Result<u64, String> {
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
        let chain = c.borrow();
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
async fn get_gas_price() -> Result<u64, String> {
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
        let chain = c.borrow();
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

// // relayer canister call this to check if a message is valid before process_message
// #[query(name = "is_valid")]
// #[candid_method(query, rename = "is_valid")]
// fn is_valid(message: Vec<u8>, proof: Vec<Vec<u8>>, leaf_index: u32) -> Result<bool, String> {
//     // verify message proof: use proof, message to calculate the merkle root, 
//     // check if the merkle root exists in corresponding chain state
//     let m = Message::from_raw(message.clone()).map_err(|e| {
//         format!("parse message from bytes failed: {:?}", e)
//     })?;
//     let h = m.to_leaf();
//     let p_h256: Vec<H256> = proof.iter().map(|v| H256::from_slice(&v)).collect();
//     let p: [H256; TREE_DEPTH] = p_h256.try_into().map_err(|e| format!("parse proof failed: {:?}", e))?;
//     // calculate root with leaf hash & proof
//     let root = merkle_root_from_branch(h, &p, TREE_DEPTH, leaf_index as usize);
//     // do not add optimistic yet
//     CHAINS.with(|c| {
//         let chain = c.borrow();
//         Ok(chain.is_root_exist(root))
//     })
// }

#[query(name = "get_last_scanned_block")]
#[candid_method(query, rename = "get_last_scanned_block")]
fn get_last_scanned_block() -> String {
    CHAINS.with(|c| {
        let chain = c.borrow();
        format!("{:x}", chain.last_scanned_block)
    })
}

#[query(name = "get_messages")]
#[candid_method(query, rename = "get_messages")]
fn get_messages(start: u64, limit: u64) -> Result<Vec<MessageStable>, String> {
    CHAINS.with(|c| {
        let chain = c.borrow();
        Ok(chain.get_messages(start, limit))
    })
}

#[query(name = "get_message_by_hash")]
#[candid_method(query, rename = "get_message_by_hash")]
fn get_message_by_hash(hash: &[u8]) -> Result<Vec<MessageStable>, String> {
    CHAINS.with(|c| {
        let chain = c.borrow();
        Ok(chain.get_message_by_hash(hash))
    })
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

async fn scan() {
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
                                state.cache_msg = HashMap::default(); // reset roots in this round
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
                    let start_block = clinet.get_suggested_start_block();
                    let end_block = clinet.get_block_number().await?;
                    let mut current_block = start_block;
                    let mut chunk_size = 20; // 
                    let mut all_events: Vec<MessageStable> = vec![];

                    while current_block <= end_block {
                        let estimate_end_block = current_block + chunk_size;
                        add_log(format!("Scanning SendMessage Event for blocks: {:?} - {:?}, chunk size: {:?}", current_block, estimate_end_block, chunk_size));

                        let (actual_end_block, chunk_events) = client.scan_chunk(current_block, estimate_end_block).await?;
                        // adjust chunk size dynamically
                        chunk_size = estimate_next_chunk_size(chunk_size, chunk_events.len());
                        all_events.extend(chunk_events);
                        current_block = actual_end_block + 1;

                    }
                    //store to cache messgae
                    incr_state(all_events);

                    // check msgs with different rpc and store to chainState
                    STATE_MACHINE.with(|s| {
                        let s = s.borrow();
                        let (check_result, _) = check_scan_message_results(&s.cache_msg, s.rpc_count());
                        s.get_fetching_next_sub_state(check_result)
                    })
                    // todo , store to chain state

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
            Task::Scan, 
            ic_cron::types::SchedulingOptions {
                delay_nano: get_scan_event_period(),
                interval_nano: get_scan_event_period(),
                iterations: Iterations::Exact(1),
            },
        ).unwrap();
    }
}

#[heartbeat]
fn heart_beat() {
    for task in cron_ready_tasks() {
        let kind = task.get_payload::<Task>().expect("Serialization error");
        match kind {
            Task::Scan => {
                ic_cdk::spawn(scan());
            }
        }
    }
}

#[pre_upgrade]
fn pre_upgrade() {
    let chains = CHAINS.with(|c| {
        c.replace(ChainState::default())
    });
    let state_info = STATE_INFO.with(|s| {
        s.replace(StateInfo::default())
    });
    let state_machine = STATE_MACHINE.with(|s| {
        s.replace(StateMachine::default())
    });
    ic_cdk::storage::stable_save((chains, state_info, state_machine, _take_cron_state())).expect("pre upgrade error");
}

#[post_upgrade]
fn post_upgrade() {
    let (chains, 
        state_info, 
        state_machine, 
        cron_state
    ): (ChainState, 
        StateInfo, 
        StateMachine, 
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

fn incr_state(msg: Vec<MessageStable>) {
    // store to STATE_MACHINE cache buffer
    // STATE_MACHINE.with(|s| {
    //     let mut state = s.borrow_mut();
    //     state
    //         .roots
    //         .entry(root)
    //         .and_modify(|c| *c += 1)
    //         .or_insert(1);
    // })
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

fn estimate_next_chunk_size(chunk_size: usize, events_count: usize) -> usize {
    let min_chunk_size = 10;
    let max_chunk_size = 30;
    let chunk_size_descrese = 0.5;
    let chunk_size_increase = 2.0;
    let mut current_chuck_size = if events_count > 0 {min_chunk_size} else {chunk_size * chunk_size_increase};

    current_chuck_size = cmp::max(min_chunk_size, current_chuck_size);
    current_chuck_size = cmp::min(max_chunk_size, current_chuck_size);
    current_chuck_size
}


#[cfg(not(any(target_arch = "wasm32", test)))]
fn main() {
    // The line below generates did types and service definition from the
    // methods annotated with `candid_method` above. The definition is then
    // obtained with `__export_service()`.
    candid::export_service!();
    std::print!("{}", __export_service());
}

#[cfg(any(target_arch = "wasm32", test))]
fn main() {}