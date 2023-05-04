/*
omnic proxy canister:
    send_message: fetch merkel roots from all supported chains and insert to chain state
    process_message: process message from gateway
*/

use std::cell::{RefCell};
use std::collections::{HashMap, VecDeque};

use ic_web3::ic::get_eth_addr;

use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update};
use ic_cdk::export::candid::{candid_method};
use ic_cdk::api::management_canister::http_request::{HttpResponse, TransformArgs};
use candid::types::principal::Principal;

use omnic::utils::DetailsBuilder;
use omnic::{Message, chains::EVMChainClient, ChainConfig, ChainState, ChainType};
use omnic::{HomeContract, DetailValue, Record};
use omnic::consts::{KEY_NAME, MAX_RESP_BYTES, CYCLES_PER_CALL, CYCLES_PER_BYTE};
use omnic::state::{StateInfo, RecordDB};
use omnic::call::{call_to_canister, call_to_chain};

#[derive(CandidType, Deserialize, Default, Clone)]
pub struct MessageCache {
    msgs: HashMap<u32, VecDeque<(u64, MessageStable)>> // chain => (timestamp, message)
} // cache messages for each chain

impl MessageCache {
    // todo: implement
    fn get_message_len(&self, chain: u32) -> usize {
        self.msgs.get(&chain).len()
    }

    fn clean_messages(&mut self, chain: u32) {
        self.msgs.get_mut(&chain).clear();
    }

    fn get_front_msg_ts(&self, chain: u32) -> {
        self.msgs.get(&chain).front().unwrap_or()
    }
}

thread_local! {
    static OWNERS: RefCell<HashSet<Principal>> = RefCell::new(HashSet::default());
    static CHAINS: RefCell<HashMap<u32, ChainState>>  = RefCell::new(HashMap::new());
    static LOGS: RefCell<VecDeque<String>> = RefCell::new(VecDeque::default());
    static RECORDS: RefCell<RecordDB> = RefCell::new(RecordDB::new());
}

#[query]
#[candid_method(query)]
fn get_logs() -> Vec<String> {
    LOGS.with(|l| {
        l.borrow().clone().into()
    })
}

#[init]
#[candid_method(init)]
fn init() {
    let caller = ic_cdk::api::caller();
    OWNERS.with(|owner| {
        owner.borrow_mut().insert();
    });
}

#[query]
async fn transform(raw: TransformArgs) -> HttpResponse {
    let mut t = raw.response;
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
#[candid_method(update, rename = "add_chain")]
fn add_chain(
    chain_id: u32, 
    urls: Vec<String>, 
    gateway_canister_addr: Principal,
    omnic_addr: String, 
    start_block: u64
) -> Result<bool, String> {
    // add chain config
    // need to deploy gateway canister manually
    // provide the gateway canister principal, as the WASM size will exceed if include the gateway canister bytes
    CHAINS.with(|chains| {
        let mut chains = chains.borrow_mut();
        if !chains.contains_key(&chain_id) {
            chains.insert(chain_id, ChainState::new(
                ChainConfig::new(
                    ChainType::Evm,
                    chain_id,
                    urls.clone(),
                    gateway_canister_addr,
                    omnic_addr.clone(),
                    start_block,
                )
            ));
        }
    });
    // add record
    add_record(
        ic_cdk::caller(), 
        "add_chain".to_string(), 
        DetailsBuilder::new()
            .insert("chain_id", DetailValue::U64(chain_id as u64))
            .insert("urls", DetailValue::Text(urls.join(",")))
            .insert("gatewat_addr", DetailValue::Principal(gateway_canister_addr))
            .insert("omnic_addr", DetailValue::Text(omnic_addr))
            .insert("start_block", DetailValue::U64(start_block))
    );
    Ok(true)
}

#[update(name = "delete_chain", guard = "is_authorized")]
#[candid_method(update, rename = "delete_chain")]
fn delete_chain(chain_id: u32) -> Result<bool, String> {
    match CHAINS.with(|c| {
        let mut chains = c.borrow_mut();
        chains.remove(&chain_id)
    }) {
        Some(_) => { 
            add_record(
                ic_cdk::caller(), 
                "delete_chain".to_string(), 
                DetailsBuilder::new()
                    .insert("chain_id", DetailValue::U64(chain_id as u64))
            );
            Ok(true) 
        }
        None => { Err("Chain id not exist".to_string()) }
    }
}

// update chain settings
#[update(guard = "is_authorized")]
#[candid_method(update, rename = "update_chain")]
fn update_chain(
    chain_id: u32, 
    urls: Vec<String>, 
    gateway_canister_addr: Principal,
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
                    urls.clone(),
                    gateway_canister_addr,
                    omnic_addr.clone(),
                    start_block,
                )
            ));
        }
    });
    add_record(
        ic_cdk::caller(), 
        "update_chain".to_string(), 
        DetailsBuilder::new()
            .insert("chain_id", DetailValue::U64(chain_id as u64))
            .insert("urls", DetailValue::Text(urls.join(",")))
            .insert("gateway_addr", DetailValue::Principal(gateway_canister_addr))
            .insert("omnic_addr", DetailValue::Text(omnic_addr))
            .insert("start_block", DetailValue::U64(start_block))
    );
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

// update chain settings
// clear existing messages cache immediately
#[update(guard = "is_authorized")]
#[candid_method(update, rename = "trigger_clear_cache")]
fn trigger_clear_cache(dst_chains: Vec<u32>) -> Result<(String, u64), String> {

}

// call by application
// cache message and send them as a batch when it reaches the maximum capacity of the cache or
// the limited time
#[update(name = "send_message")]
#[candid_method(update, rename = "send_message")]
async fn send_message(dst_chain: u32, recipient: &[u8;32], payload: &[u8]) -> Result<(String, u64), String> {
    // check cycles
    let available = ic_cdk::api::call::msg_cycles_available();
    let need_cycles = payload.len() as u64 * CYCLES_PER_BYTE;
    if available < need_cycles {
        return Err(format!("Insufficient cycles: require {} cycles. Received {}.", need_cycles, available));
    }
    // accept cycles
    let _accepted = ic_cdk::api::call::msg_cycles_accept(need_cycles);

    c
    // TODO:
    // cache message
    // charge cycles as fee
}

// only gateway canister call
#[update(name = "process_message")]
#[candid_method(update, rename = "process_message")]
async fn process_message(message: Vec<MessageStable>) -> Result<(String, u64), String> {

    // todo: add controller
    // is gateway canister?

    // send msg to destination
    // TODO reset next index after call error?
    let res = if m.destination == 0 {
        // take last 10 bytes
        for m in message.iter() {
            let recipient = Principal::from_slice(&m.recipient.as_bytes()[22..]);
            add_log(format!("recipient: {:?}", Principal::to_text(&recipient)));
            let res = call_to_canister(recipient, &m).await?;
            let time = ic_cdk::api::time();
            Ok((res, time))
        }
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
    };
    
    add_record(
        origin_caller, 
        "process_message".to_string(), 
        DetailsBuilder::new()
            .insert("origin", DetailValue::U64(m.origin as u64))
            .insert("send", DetailValue::Text(m.sender.to_string()))
            .insert("nonce", DetailValue::U64(m.nonce as u64))
            .insert("destination", DetailValue::U64(m.destination as u64))
            .insert("recipient", DetailValue::Text(m.recipient.to_string()))
            .insert("result", DetailValue::Text(
                match res.clone() {
                    Ok(o) => {
                        o
                    }
                    Err(e) => {
                        e
                    }
                }
            ))
    );
    
    res.map(|o| (o, ic_cdk::api::time()))
}

#[update(name = "add_owner", guard = "is_authorized")]
#[candid_method(update, rename = "add_owner")]
async fn add_owner(owner: Principal) {
    OWNERS.with(|o| {
        o.borrow_mut().insert(owner);
    });
}

#[update(name = "remove_owner", guard = "is_authorized")]
#[candid_method(update, rename = "remove_owner")]
async fn remove_owner(owner: Principal) {
    OWNERS.with(|o| {
        o.borrow_mut().remove(&owner);
    });
}

#[query(name = "get_record_size", guard = "is_authorized")]
#[candid_method(query, rename = "get_record_size")]
fn get_record_size(operation: Option<String>) -> usize {
    RECORDS.with(|r| {
        let records = r.borrow();
        records.size(operation)
    })
}

#[query(name = "get_record", guard = "is_authorized")]
#[candid_method(query, rename = "get_record")]
fn get_record(id: usize) -> Option<Record> {
    RECORDS.with(|r| {
        let records = r.borrow();
        records.load_by_id(id)
    })
}

#[query(name = "get_records", guard = "is_authorized")]
#[candid_method(query, rename = "get_records")]
fn get_records(range: Option<(usize, usize)>, operation: Option<String>) -> Vec<Record> {
    RECORDS.with(|r| {
        let records = r.borrow();
        let (start, end) = match range {
            Some((s, e)) => {
                (s, e)
            }
            None => {
                // range not set, default to last 50 records
                let size = records.size(operation.clone());
                if size < 50 {
                    (0, size)
                } else {
                    (size-50, size)
                }
            }
        };

        match operation {
            Some(op) => {
                // get specific operation
                records.load_by_opeation(op, start, end)
            }
            None => {
                // operation not set, get all
                records.load_by_id_range(start, end)
            }
        }
    })
}

#[pre_upgrade]
fn pre_upgrade() {
    let chains = CHAINS.with(|c| {
        c.replace(HashMap::default())
    });
    let owners = OWNERS.with(|o| {
        o.replace(HashSet::default())
    });
    let records = RECORDS.with(|r| {
        r.replace(RecordDB::new())
    });
    ic_cdk::storage::stable_save((chains, owners, records, )).expect("pre upgrade error");
}

#[post_upgrade]
fn post_upgrade() {
    let (chains, 
        owners,
        records,
    ): (HashMap<u32, ChainState>, 
        HashSet, 
        RecordDB,
    ) = ic_cdk::storage::stable_restore().expect("post upgrade error");
    
    CHAINS.with(|c| {
        c.replace(chains);
    });
    OWNERS.with(|s| {
        s.replace(owners);
    });
    RECORDS.with(|r| {
        r.replace(records);
    });
}

// get the unix timestamp in second
fn get_time() -> u64 {
    ic_cdk::api::time() / 1000000000
}

fn is_authorized() -> Result<(), String> {
    let user = ic_cdk::api::caller();
    OWNERS.with(|owner| {
        if !owner.borrow().contains(user) {
            Err("unauthorized!".into())
        } else {
            Ok(())
        }
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

fn add_record(caller: Principal, op: String, details_builder: DetailsBuilder) {
    RECORDS.with(|r| {
        let mut records = r.borrow_mut();
        records.append(
            caller, 
            get_time(), 
            op, 
            details_builder.build()
        );
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
