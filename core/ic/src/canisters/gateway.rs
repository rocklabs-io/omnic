/*
omnic proxy canister:
    fetch_root: fetch messages from all supported chains and insert to chain state
*/

use std::cell::{RefCell};
use std::collections::{HashMap, VecDeque};
use std::convert::TryInto;

use ic_cdk::api::call::CallResult;
use rand::{rngs::StdRng, SeedableRng};
use rand::seq::SliceRandom;

use ic_cdk::api::management_canister::http_request::{HttpResponse, TransformArgs};
use ic_cron::task_scheduler::TaskScheduler;

use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update, heartbeat};
use ic_cdk::export::candid::{candid_method, CandidType, Deserialize};
use candid::types::principal::Principal;

use ic_cron::types::Iterations;

use omnic::{Message, chains::EVMChainClient, ChainConfig, ChainState, ChainType};
use omnic::{HomeContract, MessageStable};
use omnic::consts::{MAX_RESP_BYTES, CYCLES_PER_CALL, CYCLES_PER_BYTE};
use omnic::state::{State, StateMachine, StateInfo};
use omnic::utils::{check_roots_result, check_scan_message_results, get_batch_next_block};

ic_cron::implement_cron!();

#[derive(CandidType, Deserialize, Clone)]
enum Task {
    FetchRoots,
    FetchRoot
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

fn get_fetch_msg_period() -> u64 {
    STATE_INFO.with(|s| s.borrow().fetch_msg_period)
}

fn get_fetch_msgs_period() -> u64 {
    STATE_INFO.with(|s| s.borrow().fetch_msgs_period)
}

fn get_confirm_block() -> u64 {
    STATE_INFO.with(|s| s.borrow().confirm_block)
}

fn get_scan_block_size() -> u64 {
    STATE_INFO.with(|s| s.borrow().scan_block_size)
}

fn get_query_rpc_number() -> u64 {
    STATE_INFO.with(|s| s.borrow().query_rpc_number)
}

#[init]
#[candid_method(init)]
fn init(proxy: Principal) {
    let caller = ic_cdk::api::caller();
    STATE_INFO.with(|info| {
        let mut info = info.borrow_mut();
        info.add_owner(caller);
        info.set_proxy_addr(proxy);
    });

    // set up cron job
    cron_enqueue(
        Task::FetchRoots, 
        ic_cron::types::SchedulingOptions {
            delay_nano: get_fetch_msgs_period(),
            interval_nano: get_fetch_msgs_period(),
            iterations: Iterations::Infinite,
        },
    ).unwrap();
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
#[candid_method(update, rename = "set_confirm_block")]
async fn set_confirm_block(confirm_block: u64) -> Result<bool, String> {
    STATE_INFO.with(|s| {
        let mut s = s.borrow_mut();
        s.set_confirm_block(confirm_block);
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
                0,
                0,
            )
        ));
    });
    STATE_MACHINE.with(|s| {
        let mut state_machine = s.borrow_mut();
        state_machine.set_chain_id(chain_id);
        state_machine.set_rpc_urls(urls.clone());
        state_machine.set_omnic_addr(omnic_addr.clone());
        state_machine.set_last_block_height(start_block-1);
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

#[query(name = "get_info", guard = "is_authorized")]
#[candid_method(query, rename = "get_info")]
fn get_info() -> Result<StateInfo, String> {
    STATE_INFO.with(|info| {
        let info = info.borrow();
        Ok(info.clone())
    })
}

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

async fn fetch_msg() {
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
                                let last_block_height = state.last_block_height;
                                if last_block_height == h {
                                    // if the block height is the same as before, skip this round
                                    add_log(format!("block height is not changed: {}", h));
                                    State::Fail
                                } else {
                                    let next_block  = get_batch_next_block(last_block_height, h, get_confirm_block(), get_scan_block_size());
                                    add_log(format!("block height update to {}", next_block));
                                    state.block_height = next_block;
                                    state.cache_msg = HashMap::default(); // reset msgs in this round
                                    State::Fetching(0)
                                }
                            })
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
            // query messags in [last block height + 1, block height]
            match EVMChainClient::new(state.rpc_urls[idx].clone(), state.omnic_addr.clone(), MAX_RESP_BYTES, CYCLES_PER_CALL) {
                Ok(client) => {
                    let scan_results: Result<Vec<MessageStable>, omnic::OmnicError> = client.scan_chunk(state.last_block_height, state.block_height).await;
                    match scan_results {
                        Ok(msgs) => {
                            add_log(format!("scan block from {} to {}, get {} events", state.last_block_height, state.block_height, msgs.len()));
                            msgs.into_iter().for_each(|msg| {
                                incr_state_message(msg);
                            });
                        },
                        Err(e) => {
                            add_log(format!("failed to scan block from {} to {}", state.last_block_height, state.block_height));
                            add_log(format!("query messages from {} failed: {}", state.rpc_urls[idx].clone(), e));
                            // increase the default message, as the count of query RPC failed
                            incr_state_message(MessageStable::default());
                        },
                    }
                    
                    STATE_MACHINE.with(|s| {
                        let s = s.borrow();
                        s.get_fetching_next_sub_state()
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
                delay_nano: get_fetch_msg_period(),
                interval_nano: get_fetch_msg_period(),
                iterations: Iterations::Exact(1),
            },
        ).unwrap();
    }
}

// this is done in heart_beat
async fn fetch_msgs() {
    let state = STATE_MACHINE.with(|s| {
        s.borrow().clone()
    });

    match state.state {
        State::Init => {
            // get chain ids
            let chain_id = CHAINS.with(|c| {
                c.borrow().config.chain_id
            });
            // when chain is set, start fetching
            if chain_id != 0 {
                STATE_MACHINE.with(|s| {
                    let mut state = s.borrow_mut();
                    state.state = State::Fetching(0);
                });
            }
        }
        State::Fetching(_) => {
            match state.sub_state {
                State::Init => {
                    // randomly select rpc url to fetch
                    // call IC raw rand to get random seed
                    let seed_res = ic_cdk::api::management_canister::main::raw_rand().await;
                    match seed_res {
                        Ok((seed, )) => {
                            let mut rpc_urls = CHAINS.with(|c| {
                                c.borrow().config.rpc_urls.clone()
                            });
                            // shuffle
                            let seed: [u8; 32] = seed.as_slice().try_into().expect("convert vector to array error");
                            let mut rng: StdRng = SeedableRng::from_seed(seed);
                            rpc_urls.shuffle(&mut rng);
                            let random_urls = rpc_urls[..get_query_rpc_number() as usize].to_vec();
                            // set random urls for this round
                            STATE_MACHINE.with(|s| {
                                s.borrow_mut().set_rpc_urls(random_urls.clone());
                            });
                            add_log(format!("start fetching, random rpc urls: {:?}", random_urls));
                            add_log(format!("start_cycles: {:?},  start_time: {:?}", ic_cdk::api::canister_balance(), ic_cdk::api::time()));
                            cron_enqueue(
                                Task::FetchRoot, 
                                ic_cron::types::SchedulingOptions {
                                    delay_nano: get_fetch_msg_period(),
                                    interval_nano: get_fetch_msg_period(),
                                    iterations: Iterations::Exact(1),
                                },
                            ).unwrap();
                        },
                        Err((_code, msg)) => {
                            // error, do nothing
                            add_log(format!("Error getting raw rand: {}", msg));
                        },
                    }
                }
                State::Fetching(_) => {},
                State::End => {
                    add_log(format!("end_cycles: {:?},  end_time: {:?}", ic_cdk::api::canister_balance(), ic_cdk::api::time()));
                    // get the valid message
                    let (check_results, valid_msgs) = check_scan_message_results(&state.cache_msg, state.rpc_count());
                    if check_results {
                        // store the message
                        CHAINS.with(|c| {
                            let mut chain = c.borrow_mut();
                            valid_msgs.iter().for_each(|msg| {
                                chain.insert_message(msg.clone());
                            });
                        });
                        // call proxy to send message
                        // TODO how to handle send failed?
                        ic_cdk::println!("valid message amount: {}", valid_msgs.len());
                        if valid_msgs.len() > 0 {
                            ic_cdk::spawn(send_message_to_proxy(valid_msgs));
                        }
                    }
                    
                    
                    // update state, and the last block number if pass
                    STATE_MACHINE.with(|s| {
                        let mut state = s.borrow_mut();
                        (state.state, state.sub_state) = state.get_fetching_next_state();
                        if check_results {
                            state.last_block_height = state.block_height;
                        }
                    });
                },
                State::Fail => {
                    // update state, dont update the last block number so that it can re-fetch again
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
                ic_cdk::spawn(fetch_msgs());
            },
            Task::FetchRoot => {
                ic_cdk::spawn(fetch_msg());
            },
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

async fn send_message_to_proxy(msgs: Vec<MessageStable>) {
    let proxy = STATE_INFO.with(|s| {
        s.borrow().proxy_addr
    });

    ic_cdk::println!("call proxy to process: {}", proxy.clone());
    let call_res: CallResult<(Result<Vec<(String, u64)>, String>,)> = ic_cdk::call(proxy, "process_message", (msgs, )).await;
    if call_res.is_err() {
        add_log(format!("call proxy failed: {:?}", call_res.err()))
    }
}

fn incr_state_message(msg: MessageStable) {
    STATE_MACHINE.with(|s| {
        let mut state = s.borrow_mut();
        state
            .cache_msg
            .entry(msg)
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