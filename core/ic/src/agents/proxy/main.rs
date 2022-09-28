/*
omnic proxy canister:
    fetch_root: fetch merkel roots from all supported chains and insert to chain state
*/

use std::cell::{RefCell};
use std::collections::HashMap;
use std::str::FromStr;
use std::convert::TryInto;

use ic_web3::Web3;
use ic_web3::contract::{Contract, Options};
use ic_web3::types::{H256, Address, BlockNumber, BlockId};
use ic_web3::transports::ICHttp;
use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update, heartbeat};
use ic_cdk::export::candid::{candid_method, CandidType, Deserialize};
use ic_cdk::api::call::{call, CallResult};
use ic_cdk::export::Principal;

use ic_cron::types::Iterations;

use accumulator::{MerkleProof, Proof, TREE_DEPTH, merkle_root_from_branch};
use omnic::{Message, chains::{ChainRoots}};
use omnic::Decode;

ic_cron::implement_cron!();

const OPTIMISTIC_DELAY: u64 = 1800; // 30 mins
const FETCH_ROOTS_PERIOID: u64 = 1_000_000_000 * 30; //60 * 5; // 5 min in nano second
const FETCH_ROOT_PERIOID: u64 = 1_000_000_000 * 5; //10; // 1 min in nano second

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
    pub fn set_chains(&mut self, ids: Vec<u32>, rpc_urls: Vec<String>) {
        self.chain_ids = ids;
        self.rpc_urls = rpc_urls;
    }
}

const OMNIC_ABI: &[u8] = include_bytes!("./omnic.abi");

const GOERLI_CHAIN_ID: u32 = 5;
const GOERLI_URL: &str = "https://eth-goerli.g.alchemy.com/v2/0QCHDmgIEFRV48r1U1QbtOyFInib3ZAm";
const GOERLI_OMNIC_ADDR: &str = "0312504E22B40A6f03FcCFEA0C8c0e9Ad3E36918";
const GOERLI_START_BLOCK: u64 = 7558863;

thread_local! {
    static CHAINS: RefCell<HashMap<u32, ChainRoots>>  = RefCell::new(HashMap::new());
    static STATE_MACHINE: RefCell<StateMachine> = RefCell::new(StateMachine::default())
}

#[init]
#[candid_method(init)]
fn init() {
    // add goerli chain config
    CHAINS.with(|chains| {
        let mut chains = chains.borrow_mut();
        // ledger.init_metadata(ic_cdk::caller(), args.clone());
        chains.insert(GOERLI_CHAIN_ID, ChainRoots::new(
            GOERLI_CHAIN_ID,
            vec![GOERLI_URL.clone().into()],
            GOERLI_OMNIC_ADDR.clone().into(),
            GOERLI_START_BLOCK,
            Some(1000),
        ));
    });

    // init state machine
    STATE_MACHINE.with(|s| {
        let mut state_machine = s.borrow_mut();
        // append s.chain_ids;
        state_machine.set_chains(
            vec![GOERLI_CHAIN_ID],
            vec![GOERLI_URL.to_string()]
        );
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
fn is_valid(message: Vec<u8>, proof: Vec<Vec<u8>>, leaf_index: u32) -> Result<bool, String> {
    // verify message proof: use proof, message to calculate the merkle root, 
    //     if message.need_verify is false, we only check if root exist in the hashmap
    //     if message.need_verify is true, we additionally check root.confirm_at <= ic_cdk::api::time()
    // let m: Message = serde_json::from_str(message.as_str()).map_err(|e| {
    //     format!("error in parse message json: {:?}", e)
    // })?;
    // ic_cdk::println!("msg: {:?}, proof: {:?}, leaf_index: {:?}", hex::encode(&message.clone()), proof.clone(), leaf_index);
    let m = Message::read_from(&mut message.clone().as_slice()).map_err(|e| {
        format!("error in parse message json: {:?}", e)
    })?;
    let h = m.to_leaf();
    // let p: [H256; TREE_DEPTH] = serde_json::from_str(proof.as_str()).map_err(|e| {
    //     format!("error in parse proof json: {:?}", e)
    // })?;
    let p_h256: Vec<H256> = proof.iter().map(|v| H256::from_slice(&v)).collect();
    let p: [H256; TREE_DEPTH] = p_h256.try_into().map_err(|e| format!("convert to proof failed: {:?}", e))?;
    // calculate root with leaf hash & proof
    let root = merkle_root_from_branch(h, &p, TREE_DEPTH, leaf_index as usize);
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
    // TODO only relayers can call?
    // verify message proof: use proof, message to calculate the merkle root, 
    //     if message.need_verify is false, we only check if root exist in the hashmap
    //     if message.need_verify is true, we additionally check root.confirm_at <= ic_cdk::api::time()
    // if valid, call dest canister.handleMessage or send tx to dest chain
    // if invalid, return error
    let valid = is_valid(message.clone(), proof, leaf_index)?;
    if !valid {
        ic_cdk::println!("message does not pass verification!");
        return Err("message does not pass verification!".into());
    }
    // let m: Message = serde_json::from_str(message.as_str()).map_err(|e| {
    //     format!("error in parse message json: {:?}", e)
    // })?;
    let m = Message::read_from(&mut message.clone().as_slice()).map_err(|e| {
        format!("error in parse message json: {:?}", e)
    })?;
    // take last 10 bytes
    let recipient = Principal::from_slice(&m.recipient.as_bytes()[22..]);
    let sender = m.sender.as_bytes();
    ic_cdk::println!("recipient: {:?}", Principal::to_text(&recipient));
    if m.destination == 0 {
        // todo! call ic canister
        let ret: CallResult<(Result<bool, String>,)> = 
            call(recipient, "handle_message", (m.origin, m.nonce, sender, m.body, )).await;
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
            },
            Err((_code, msg)) => {
                ic_cdk::println!("call app canister failed: {:?}", (_code, msg));
            }
        }
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

    let max_resp_bytes = Some(300);
    let cycles_per_call = None;
    
    let next_state = match state.sub_state {
        State::Init => {
            match ICHttp::new(&state.rpc_urls[0], max_resp_bytes, cycles_per_call) {
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
            match ICHttp::new(&state.rpc_urls[idx], max_resp_bytes, cycles_per_call) {
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
                            ic_cdk::println!("root: {:?}", root);
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