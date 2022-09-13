/*
omnic proxy canister:
    fetch_root: fetch merkel roots from all supported chains and insert to chain state
*/

use std::cell::{RefCell};
use std::collections::HashMap;

use ic_web3::types::{H256, U256};
use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update, heartbeat};
use ic_cdk::export::candid::{candid_method, CandidType, Deserialize, Int, Nat};

use ic_cron::task_scheduler::TaskScheduler;
use ic_cron::types::Iterations;

use accumulator::{MerkleProof, Proof, TREE_DEPTH};
use omnic::chain;
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
    root: H256,
    state: State,
    sub_state: State
}

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

    // TODO init state machine

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
            // TODO fetch height
            State::Fetching(0)
        },
        State::Fetching(idx) => {
            // TODO query root in block height
            // compare and set the result with root
            // if result != state.root, convert to fail
            if idx + 1 == state.rpc_urls.len() {
                State::End
            } else {
                State::Fetching(idx + 1)
            }
        },
        State::End => State::End,
        State::Fail => State::Fail,
    };

    // update sub state
    STATE_MACHINE.with(|s| {
        s.borrow_mut().sub_state = next_state;
    });

    cron_enqueue(
        Task::FetchRoot, 
        ic_cron::types::SchedulingOptions {
            delay_nano: FETCH_ROOT_PERIOID,
            interval_nano: FETCH_ROOT_PERIOID,
            iterations: Iterations::Infinite,
        },
    ).unwrap();
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
                    let rpc_urls = CHAINS.with(|c| {
                        c.borrow().get(&chain_id).unwrap().config.rpc_urls.clone()
                    });
                    STATE_MACHINE.with(|s| {
                        let mut state = s.borrow_mut();
                        state.rpc_urls = rpc_urls;
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

// #[heartbeat]
// fn heart_beat() {

// }

/// get the unix timestamp in second
fn get_time() -> u64 {
    ic_cdk::api::time() / 1000000000
}

fn main() {}