/*
omnic proxy canister:
    fetch_root: fetch merkel roots from all supported chains and insert to chain state
*/

use std::cell::{RefCell};
use std::collections::HashMap;
use std::str::FromStr;

use ic_web3::Web3;
use ic_web3::contract::{Contract, Options};
use ic_web3::types::{H256, Address, BlockNumber, BlockId};
use ic_web3::transports::ICHttp;
use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update, heartbeat};
use ic_cdk::export::candid::{candid_method, CandidType, Deserialize};

use ic_cron::types::Iterations;

use accumulator::{MerkleProof, Proof, TREE_DEPTH};
use omnic::{Message, chains::{ChainRoots}};

ic_cron::implement_cron!();

const OPTIMISTIC_DELAY: u64 = 1800; // 30 mins
const FETCH_ROOTS_PERIOID: u64 = 1_000_000_000 * 60 * 5; // 5 min in nano second
const FETCH_ROOT_PERIOID: u64 = 1_000_000_000 * 10; // 1 min in nano second

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

const OMNIC_ABI: &[u8] = include_bytes!("./omnic.abi");

thread_local! {
    static CHAINS: RefCell<HashMap<u32, ChainRoots>>  = RefCell::new(HashMap::new());
    static STATE_MACHINE: RefCell<StateMachine> = RefCell::new(StateMachine::default())
}

#[init]
#[candid_method(init)]
fn init() {
    // add goerli chain config
    // CHAINS.with(|chains| {
    //     let mut chains = chains.borrow_mut();
    //     // ledger.init_metadata(ic_cdk::caller(), args.clone());
    //     chains.insert(GOERLI_CHAIN_ID, ChainConfig {
    //         chain_id: GOERLI_CHAIN_ID,
    //         rpc_url: GOERLI_URL.clone().into(),
    //         omnic_addr:GOERLI_OMNIC_ADDR.clone().into(),
    //         omnic_start_block: 7468220,
    //         current_block: 7468220,
    //         batch_size: 1000,
    //     });
    // });

    // init state machine
    STATE_MACHINE.with(|s| {
        let mut state_machine = s.borrow_mut();
        // append s.chain_ids;
    });

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

// relayer canister call this to check if a message is valid before process_message
#[query(name = "is_valid")]
#[candid_method(query, rename = "is_valid")]
fn is_valid(proof: String, message: String) -> Result<bool, String> {
    // verify message proof: use proof, message to calculate the merkle root, 
    //     if message.need_verify is false, we only check if root exist in the hashmap
    //     if message.need_verify is true, we additionally check root.confirm_at <= ic_cdk::api::time()
    let m: Message = serde_json::from_str(message.as_str()).map_err(|e| {
        format!("error in parse message json: {:?}", e)
    })?;
    let h = m.to_leaf();
    let p: Proof<{ TREE_DEPTH }> = serde_json::from_str(proof.as_str()).map_err(|e| {
        format!("error in parse proof json: {:?}", e)
    })?;
    assert_eq!(h, p.leaf);
    let root = p.root();
    if m.wait_optimistic {
        let now = get_time();
        CHAINS.with(|c| {
            let chains = c.borrow();
            let chain = chains.get(&m.origin).ok_or("src chain id not exist".to_string())?;
            Ok(chain.is_root_valid(root, now))
        })
    } else {
        CHAINS.with(|c| {
            let chains = c.borrow();
            let chain = chains.get(&m.origin).ok_or("src chain id not exist".to_string())?;
            Ok(chain.is_root_exist(root))
        })
    }
}

#[update(name = "process_message")]
#[candid_method(update, rename = "process_message")]
async fn process_message(proof: String, message: String) -> Result<bool, String> {
    // TODO only relayers can call?
    // verify message proof: use proof, message to calculate the merkle root, 
    //     if message.need_verify is false, we only check if root exist in the hashmap
    //     if message.need_verify is true, we additionally check root.confirm_at <= ic_cdk::api::time()
    // if valid, call dest canister.handleMessage or send tx to dest chain
    // if invalid, return error
    is_valid(proof, message.clone())?;
    let m: Message = serde_json::from_str(message.as_str()).map_err(|e| {
        format!("error in parse message json: {:?}", e)
    })?;
    if m.destination == 0 {
        // todo! call ic canister
    } else {
        // todo! send tx to chain
    }
    Ok(true)
}

async fn fetch_root() {
    // query omnic contract.getLatestRoot, 
    // fetch from multiple rpc providers and aggregrate results, should be exact match
    let state = STATE_MACHINE.with(|s| {
        s.borrow().clone()
    });
    
    let next_state = match state.sub_state {
        State::Init => {
            match ICHttp::new(&state.rpc_urls[0], None, None) {
                Ok(v) => { 
                    let w3 = Web3::new(v);
                    match w3.eth().block_number().await {
                        Ok(h) => {
                            STATE_MACHINE.with(|s| {
                                s.borrow_mut().block_height = h.as_u64();
                            });
                            State::Fetching(0)
                        },
                        Err(e) => {
                            ic_cdk::println!("init contract failed: {}", e);
                            State::Fail
                        },
                    }
                },
                Err(e) => { 
                    State::Fail
                },
            }
        },
        State::Fetching(idx) => {
            // query root in block height
            match ICHttp::new(&state.rpc_urls[idx], None, None) {
                Ok(v) => {
                    let w3 = Web3::new(v);
                    let contract_address = Address::from_str(&state.omnic_addr).unwrap();
                    let contract = Contract::from_json(
                        w3.eth(),
                        contract_address,
                        OMNIC_ABI
                    );
                    match contract {
                        Ok(c) => {
                            let root: Result<H256, ic_web3::contract::Error> = c
                                .query("getLatestRoot", (), None, Options::default(), BlockId::Number(BlockNumber::Number(state.block_height.into())))
                                .await;
                            match root {
                                Ok(r) => {
                                    if idx == 0 {
                                        STATE_MACHINE.with(|s| {
                                            s.borrow_mut().root = r;
                                        });
                                        if idx + 1 == state.rpc_urls.len() {
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
                                                if idx + 1 == state.rpc_urls.len() {
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
                            ic_cdk::println!("init contract failed: {}", e);
                            State::Fail
                        },
                    }
                },
                Err(e) => {
                    ic_cdk::println!("init ic http failed: {}", e);
                    State::Fail
                },
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
                state.state = State::Fetching(0);
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
                        let chain_root = chain.get_mut(&state.chain_ids[idx as usize]).expect("chain id not exist");
                        chain_root.insert_root(state.root, get_time());
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
fn get_time() -> u64 {
    ic_cdk::api::time() / 1000000000
}

fn main() {
    candid::export_service!();
    std::print!("{}", __export_service());
}