/*
omnic proxy canister:
    fetch_root: fetch merkel roots from all supported chains and insert to chain state
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
use omnic::{HomeContract, DetailValue};
use omnic::consts::{KEY_NAME, MAX_RESP_BYTES, CYCLES_PER_CALL, CYCLES_PER_BYTE};
use omnic::state::{StateInfo, RecordDB};
use omnic::call::{call_to_canister, call_to_chain};

thread_local! {
    static STATE_INFO: RefCell<StateInfo> = RefCell::new(StateInfo::default());
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
    STATE_INFO.with(|info| {
        let mut info = info.borrow_mut();
        info.add_owner(caller);
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
    add_record(
        ic_cdk::caller(), 
        "set_next_index".to_string(), 
        DetailsBuilder::new()
            .insert("chain_id", DetailValue::U64(chain_id as u64))
            .insert("next_index", DetailValue::U64(next_index as u64))
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

#[update(name = "fetch_root")]
#[candid_method(update, rename = "fetch_root")]
async fn fetch(chain_id: u32, height: u64) -> Result<(String, u64, u64), String> {
    let (_caller, omnic_addr, rpc) = CHAINS.with(|chains| {
        let chains = chains.borrow();
        let c = chains.get(&chain_id).expect("chain not found");
        (c.canister_addr.clone(), c.config.omnic_addr.clone(), c.config.rpc_urls[0].clone())
    });

    let client = EVMChainClient::new(rpc, omnic_addr, MAX_RESP_BYTES, CYCLES_PER_CALL)
        .map_err(|e| format!("init client failed: {:?}", e))?;
    
    let start_cycles = ic_cdk::api::canister_balance();
    let start_time = ic_cdk::api::time();

    let root = client.get_latest_root(Some(height))
        .await
        .map(|v| hex::encode(v))
        .map_err(|e| format!("get root err: {:?}", e))?;
    
    let end_cycles = ic_cdk::api::canister_balance();
    let end_time = ic_cdk::api::time();

    let cycle_cost = start_cycles - end_cycles;
    let time_cost = end_time - start_time;
    Ok((root, cycle_cost, time_cost))
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
#[update(name = "is_valid")]
#[candid_method(query, rename = "is_valid")]
async fn is_valid(message: Vec<u8>, proof: Vec<Vec<u8>>, leaf_index: u32) -> Result<bool, String> {
    // verify message proof: use proof, message to calculate the merkle root, 
    // check if the merkle root exists in corresponding chain state
    let m = Message::from_raw(message.clone()).map_err(|e| {
        format!("parse message from bytes failed: {:?}", e)
    })?;
    // call to gate way canister
    let gateway: Principal = CHAINS.with(|c| {
        let chains = c.borrow();
        let chain = chains.get(&m.origin).ok_or("src chain id not exist".to_string())?;
        Ok::<Principal, String>(chain.config.gateway_addr)
    })?;

    let res = ic_cdk::call(gateway, "is_valid", (message, proof, leaf_index, )).await;
    match res {
        Ok((validation_result, )) => {
            Ok(validation_result)
        }
        Err((_code, msg)) => {
            Err(msg)
        }
    }
}

#[update(name = "get_latest_root")]
#[candid_method(query, rename = "get_latest_root")]
async fn get_latest_root(chain_id: u32) -> Result<String, String> {
    let gateway: Principal = CHAINS.with(|c| {
        let chains = c.borrow();
        let chain = chains.get(&chain_id).ok_or("chain id not exist".to_string())?;
        Ok::<Principal, String>(chain.config.gateway_addr)
    })?;

    let res = ic_cdk::call(gateway, "get_latest_root", ()).await;
    match res {
        Ok((root, )) => {
            Ok(root)
        }
        Err((_code, msg)) => {
            Err(msg)
        }
    }
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
async fn process_message(message: Vec<u8>, proof: Vec<Vec<u8>>, leaf_index: u32) -> Result<(String, u64), String> {
    // verify message proof: use proof, message to calculate the merkle root, 
    // check if the root exists in corresponding chain state
    // if valid, call dest canister.handleMessage or send tx to dest chain
    // if invalid, return error
    let valid = is_valid(message.clone(), proof, leaf_index).await?;
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
    // TODO reset next index after call error?
    let res = if m.destination == 0 {
        // take last 10 bytes
        let recipient = Principal::from_slice(&m.recipient.as_bytes()[22..]);
        add_log(format!("recipient: {:?}", Principal::to_text(&recipient)));
        call_to_canister(recipient, &m).await?
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
        call_to_chain(caller, omnic_addr, rpc, m.destination, message).await?
    };
    
    add_record(
        ic_cdk::caller(), 
        "process_message".to_string(), 
        DetailsBuilder::new()
            .insert("origin", DetailValue::U64(m.origin as u64))
            .insert("send", DetailValue::Text(m.sender.to_string()))
            .insert("nonce", DetailValue::U64(m.nonce as u64))
            .insert("destination", DetailValue::U64(m.destination as u64))
            .insert("recipient", DetailValue::Text(m.recipient.to_string()))
    );
    Ok((res, ic_cdk::api::time()))
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

#[pre_upgrade]
fn pre_upgrade() {
    let chains = CHAINS.with(|c| {
        c.replace(HashMap::default())
    });
    let state_info = STATE_INFO.with(|s| {
        s.replace(StateInfo::default())
    });
    let records = RECORDS.with(|r| {
        r.replace(RecordDB::new())
    });
    ic_cdk::storage::stable_save((chains, state_info, records, )).expect("pre upgrade error");
}

#[post_upgrade]
fn post_upgrade() {
    let (chains, 
        state_info,
        records,
    ): (HashMap<u32, ChainState>, 
        StateInfo, 
        RecordDB,
    ) = ic_cdk::storage::stable_restore().expect("post upgrade error");
    
    CHAINS.with(|c| {
        c.replace(chains);
    });
    STATE_INFO.with(|s| {
        s.replace(state_info);
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
    STATE_INFO.with(|info| {
        let info = info.borrow();
        if !info.is_owner(user) {
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