use std::str::FromStr;
use std::cell::RefCell;
use std::str;
use std::collections::HashMap;

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
const GOERLI_URL: &str = "https://eth-goerli.g.alchemy.com/v2/0QCHDmgIEFRV48r1U1QbtOyFInib3ZAm";
const GOERLI_CHAIN_ID: u32 = 5;
const GOERLI_OMNIC_ADDR: &str = "C3bfE8E4f99C13eb8f92C944a89C71E7be178A6F";
const EVENT_ENQUEUE_MSG: &str = "b9bede5465bf01e11c8b770ae40cbae2a14ace602a176c8ea626c9fb38a90bd8";

const IC_CHAIN_ID: u32 = 0;
const KEY_NAME: &str = "dfx_test_key";
const OMNIC_ABI: &[u8] = include_bytes!("./omnic.json");

type Result<T, E> = std::result::Result<T, E>;

ic_cron::implement_cron!();

#[derive(CandidType, Deserialize, Default)]
struct State {
    chains: HashMap<u32, ChainConfig>, // supported chains
    cron_state: Option<TaskScheduler>,
}

thread_local! {
    static CHAINS: RefCell<HashMap<u32, ChainConfig>> = RefCell::new(HashMap::new());
    // / outgoing tx queue, chainid -> Vec<signed tx data>, batch send
    // static TXS: RefCell<HashMap<u32, Vec<Vec<u8>>>> = RefCell::new(HashMap::new());
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
    // add goerli chain config
    CHAINS.with(|chains| {
        let mut chains = chains.borrow_mut();
        // ledger.init_metadata(ic_cdk::caller(), args.clone());
        chains.insert(GOERLI_CHAIN_ID, ChainConfig {
            chain_id: GOERLI_CHAIN_ID,
            rpc_url: GOERLI_URL.clone().into(),
            omnic_addr:GOERLI_OMNIC_ADDR.clone().into(),
            omnic_start_block: 7426080,
            current_block: 7426080, 
            batch_size: 1000,
        });
    });
}

#[update(name = "get_logs")]
#[candid_method(update, rename = "get_logs")]
async fn get() {
    traverse_chains().await;
}

// process message
async fn process_msg(chain: &ChainConfig, msg: &Message) -> Result<bool, String> {
    if msg.dst_chain == IC_CHAIN_ID {
        // TODO: call reciver canister.handle_message
        ic_cdk::println!("msg to ic: {:?}", msg);
    } else {
        // TODO: batch process txs
        ic_cdk::println!("msg to chain: {:?}, {:?}", msg.dst_chain, msg);
        let http = ICHttp::new(&chain.rpc_url, None).map_err(|e| format!("init ic http client failed: {:?}", e))?;
        let w3 = Web3::new(http);
        let derivation_path = vec![ic_cdk::id().as_slice().to_vec()];
        let key_info = KeyInfo{ derivation_path: derivation_path, key_name: KEY_NAME.to_string() };
        let contract_address = Address::from_str(&chain.omnic_addr).unwrap();
        let contract = Contract::from_json(
            w3.eth(),
            contract_address,
            OMNIC_ABI
        ).map_err(|e| format!("init contract failed: {}", e))?;

        let canister_addr = get_eth_addr(None, None, KEY_NAME.to_string())
            .await
            .map_err(|e| format!("get canister eth addr failed: {}", e))?;
        // add nonce to options
        let tx_count = w3.eth()
            .transaction_count(canister_addr, None)
            .await
            .map_err(|e| format!("get tx count error: {}", e))?;
        // get gas_price
        let gas_price = w3.eth()
            .gas_price()
            .await
            .map_err(|e| format!("get gas_price error: {}", e))?;
        // legacy transaction type is still ok
        let options = Options::with(|op| { 
            op.nonce = Some(tx_count);
            op.gas_price = Some(gas_price);
            op.transaction_type = Some(U64::from(2)) //EIP1559_TX_ID
        });
        // let txhash = contract
        //     .signed_call("processMessage", (to_addr, value,), options, key_info, CHAIN_ID)
        //     .await
        //     .map_err(|e| format!("tx sign failed: {}", e))?;
    }
    Ok(true)
}

async fn traverse_chains() {
    let chains = CHAINS.with(|chains| {
        chains.borrow().clone()
    });
    for (id, chain) in chains.into_iter() {
        let msgs = get_chain_msgs(&chain).await.unwrap_or_default();
        // process messages
        for msg in msgs {
            process_msg(&chain, &msg).await;
        }
    }
}

// async fn send_txs()

async fn get_chain_msgs(chain: &ChainConfig) -> Result<Vec<Message>, String> {
    let http = ICHttp::new(&chain.rpc_url, None).map_err(|e| format!("init ic http client failed: {:?}", e))?;
    let w3 = Web3::new(http);

    let block_height: u64 = w3
        .eth().block_number().await
        .map(|h| h.as_u64())
        .map_err(|e| format!("get block height err: {:?}", e))?;
    let to_block = if chain.current_block + chain.batch_size < block_height {
        chain.current_block + chain.batch_size
    } else {
        block_height
    };
    let filter = FilterBuilder::default()
        .address(vec![H160::from_str(&chain.omnic_addr).unwrap()])
        .topics(
            Some(vec![H256::from_str(EVENT_ENQUEUE_MSG).unwrap()]),
            None,
            None,
            None,
        )
        .from_block(BlockNumber::Number(chain.current_block.into()))
        .to_block(BlockNumber::Number(U64::from(to_block)))
        .build();
    let logs = w3.eth().logs(filter).await.map_err(|e| format!("get logs failed for chain: {:?}, {:?}", chain, e))?;
    // update chainconfig.current_block
    CHAINS.with(|chains| {
        let mut chains = chains.borrow_mut();
        chains.get_mut(&chain.chain_id).unwrap().current_block = to_block;
    });
    Ok(logs.iter().map(|log| Message::from_log(&log).unwrap()).collect())
}

/*
heartbeat tasks:
    GetLogs: fetch logs from supported chains and enqueue messages
    ProcessMsgs: process messages from msg queue, 
        if destination is IC canister, call canister.handleMessage
        if destination is EVM chain, construct & sign the tx and enqueue to tx queue
    SendTxs: send pending txs to external EVM chains
*/
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