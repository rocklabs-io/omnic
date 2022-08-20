use std::str::FromStr;
use std::cell::RefCell;
use std::str;

use omnic::types::{InitArgs, ChainConfig, Message, Task};

use ic_cdk::api::{call::CallResult, canister_balance};
use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update, heartbeat};
use ic_cdk::export::candid::{candid_method, CandidType, Deserialize, Int, Nat};
use ic_cdk::export::Principal;

use ic_cron::task_scheduler::TaskScheduler;
use ic_cron::types::Iterations;

use ic_web3::transports::ICHttp;
use ic_web3::Web3;
use ic_web3::ic::{get_eth_addr, KeyInfo};
use ic_web3::{
    contract::{Contract, Options},
    ethabi::ethereum_types::{U64, U256, H256, H160},
    ethabi::{Event, EventParam, ParamType, Log as ABILog, RawLog},
    types::{Address, TransactionParameters, BlockId, BlockNumber, FilterBuilder, Log},
};

// goerli testnet rpc url
// const URL: &str = "https://goerli.infura.io/v3/93ca33aa55d147f08666ac82d7cc69fd";
const URL: &str = "https://eth-goerli.g.alchemy.com/v2/0QCHDmgIEFRV48r1U1QbtOyFInib3ZAm";
const CHAIN_ID: u64 = 5;
const ETH_OMNIC_ADDR: &str = "C3bfE8E4f99C13eb8f92C944a89C71E7be178A6F";
const EVENT_ENQUEUE_MSG: &str = "b9bede5465bf01e11c8b770ae40cbae2a14ace602a176c8ea626c9fb38a90bd8";

const KEY_NAME: &str = "dfx_test_key";
// const TOKEN_ABI: &[u8] = include_bytes!("../src/contract/res/token.json");

type Result<T, E> = std::result::Result<T, E>;

ic_cron::implement_cron!();

#[derive(CandidType, Deserialize, Default)]
struct State {
    chains: Vec<ChainConfig>, // supported chains
    cron_state: Option<TaskScheduler>,
}

thread_local! {
    static CHAINS: RefCell<Vec<ChainConfig>> = RefCell::new(Vec::new());
}

#[init]
#[candid_method(init)]
fn init() {
    cron_enqueue(
        Task::GetLogs,
        ic_cron::types::SchedulingOptions {
            delay_nano: 10_000_000_000,
            interval_nano: 10_000_000_000, // every 10 seconds
            iterations: Iterations::Infinite,
        },
    ).unwrap();
}

#[update(name = "get_logs")]
#[candid_method(update, rename = "get_logs")]
async fn get() {
    get_logs().await
}

fn parse_event_enqueue_msg(log: &Log) {
    let params = vec![
        EventParam { name: "messageHash".to_string(), kind: ParamType::FixedBytes(32), indexed: true },
        EventParam { name: "dstNonce".to_string(), kind: ParamType::Uint(32), indexed: true },
        EventParam { name: "srcChainId".to_string(), kind: ParamType::Uint(32), indexed: false },
        EventParam { name: "srcSenderAddress".to_string(), kind: ParamType::FixedBytes(32), indexed: false },
        EventParam { name: "dstChainId".to_string(), kind: ParamType::Uint(32), indexed: false },
        EventParam { name: "recipient".to_string(), kind: ParamType::FixedBytes(32), indexed: false },
        EventParam { name: "data".to_string(), kind: ParamType::Bytes, indexed: false }
    ];

    let event = Event {
        name: "EnqueueMessage".to_string(),
        inputs: params,
        anonymous: false
    };
    ic_cdk::println!("event signature: {}", event.signature());
    let res = event.parse_log(RawLog {
        topics: log.topics.clone(),
        data: log.data.clone().0
    }).unwrap();
    ic_cdk::println!("parsed log: {}", serde_json::to_string(&res).unwrap());
    let data = res.params.iter().find(|p| p.name == "data").unwrap();
    ic_cdk::println!("message from Ethereum: {}", str::from_utf8(&data.value.clone().into_bytes().unwrap()).unwrap());
}

async fn get_logs() {
    ic_cdk::println!("getting logs now");
    // ic_cdk::println!("{}", H160::from_str("25816551e0e2e6fc256a0e7bcffdfd1ca3cd390d").unwrap());
    // should traverse all chains, testing only ETH now
    // filter for events in our contract
    let filter = FilterBuilder::default()
        .address(vec![H160::from_str(ETH_OMNIC_ADDR).unwrap()])
        .topics(
            Some(vec![H256::from_str(EVENT_ENQUEUE_MSG).unwrap()]),
            None,
            None,
            None,
        )
        .from_block(BlockNumber::Number(7426080.into())) // omnic contract deploy block id
        // .to_block(BlockNumber::Number(7426080.into()))
        .to_block(BlockNumber::Latest)
        .build();
    let w3 = match ICHttp::new(URL, None) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { panic!() },
    };
    let logs = w3.eth().logs(filter).await.unwrap();
    for log in logs {
        ic_cdk::println!("{}", serde_json::to_string(&log).unwrap());
        // parse into Message
        parse_event_enqueue_msg(&log);
    }
}

#[heartbeat]
fn heartbeat() {
    for task in cron_ready_tasks() {
        let kind = task.get_payload::<Task>().expect("Serialization error");
        match kind {
            Task::GetLogs => todo!(),
            SendTx => todo!()
        }
    }
}

fn get_time() -> u64 {
    ic_cdk::api::time() / 1000000000
}

// fn is_custodian() -> bool {
    
// }

// fn is_operator(sender: &Principal) -> bool {
    
// }

fn main() {
    candid::export_service!();
    std::print!("{}", __export_service());
}