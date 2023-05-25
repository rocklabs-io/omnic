/*
omnic proxy canister:
    send_message: fetch merkel roots from all supported chains and insert to chain state
    process_message: process message from gateway
*/

use std::cell::{ RefCell };
use std::collections::{ HashMap, HashSet, VecDeque };

use ic_web3::ic::get_eth_addr;
use ic_web3::types::H256;
use ic_cdk_macros::{ init, post_upgrade, pre_upgrade, query, update };
use ic_cdk::export::candid::{ candid_method, CandidType, Deserialize };
use ic_cdk::api::management_canister::http_request::{ HttpResponse, TransformArgs };
use candid::types::principal::Principal;

use omnic::utils::DetailsBuilder;
use omnic::{ chains::EVMChainClient, ChainConfig, ChainState, ChainType };
use omnic::{ HomeContract, DetailValue, Record };
use omnic::consts::{ KEY_NAME, MAX_RESP_BYTES, CYCLES_PER_CALL, CYCLES_PER_BYTE };
use omnic::state::{ StateInfo, RecordDB };
use omnic::types::{ Message, MessageType, MessageStable, encode_body };
use omnic::call::{ call_to_canister, call_to_chain };

#[derive(Default, Clone)]
pub struct MessageCache {
    msgs: HashMap<u32, VecDeque<(u64, Message)>>, // chain => (timestamp, message)
} // cache messages for each chain

impl MessageCache {
    fn get_cache_messages_len(&self, chain: u32) -> u64 {
        self.msgs.get(&chain).map_or(0, |x| x.len() as u64)
    }

    fn get_front_msg_ts(&self, chain: u32) -> u64 {
        self.msgs.get(&chain).map_or(0u64, |msg| msg.front().map_or(0u64, |item| item.0))
    }

    fn clean_messages(&mut self, chain: u32) {
        self.msgs.get_mut(&chain).map(|msg| msg.clear());
    }

    fn drain_messages(&mut self, chain: u32) -> Vec<Vec<u8>> {
        let drained: VecDeque<(u64, Message)> = self.msgs
            .get_mut(&chain)
            .map(|msg| { msg.drain(..).collect::<VecDeque<_>>() })
            .unwrap_or(VecDeque::new());
        drained
            .iter()
            .map(|m| encode_body(&m.1))
            .collect()
    }

    fn encode_msgs(&self, chain: u32) -> Vec<Vec<u8>> {
        self.msgs.get(&chain).map_or(vec![], |msg|
            msg
                .clone()
                .into_iter()
                .map(|(_, msg)| encode_body(&msg))
                .collect()
        )
    }

    fn insert_message(&mut self, chain: u32, msg: Message) {
        let m: (u64, Message) = (get_time(), msg);
        self.msgs
            .entry(chain)
            .and_modify(|msgs| (*msgs).push_back(m.clone()))
            .or_insert(VecDeque::from([m,]));
    }
}

thread_local! {
    static OWNERS: RefCell<HashSet<Principal>> = RefCell::new(HashSet::default());
    static CHAINS: RefCell<HashMap<u32, ChainState>> = RefCell::new(HashMap::new());
    static LOGS: RefCell<VecDeque<String>> = RefCell::new(VecDeque::default());
    static RECORDS: RefCell<RecordDB> = RefCell::new(RecordDB::new());
    static CACHE: RefCell<MessageCache> = RefCell::new(MessageCache::default());
}

#[query]
#[candid_method(query)]
fn get_logs() -> Vec<String> {
    LOGS.with(|l| { l.borrow().clone().into() })
}

#[init]
#[candid_method(init)]
fn init() {
    let caller = ic_cdk::api::caller();
    OWNERS.with(|owner| {
        owner.borrow_mut().insert(caller);
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
        ChainType::Evm =>
            match get_eth_addr(Some(cid), Some(derivation_path), KEY_NAME.to_string()).await {
                Ok(addr) => { Ok(hex::encode(addr)) }
                Err(e) => { Err(e) }
            }
        _ => Err("chain type not supported yet!".into()),
    }
}

#[update(guard = "is_authorized")]
#[candid_method(update, rename = "set_canister_addrs")]
async fn set_canister_addrs() -> Result<bool, String> {
    let cid = ic_cdk::id();
    let derivation_path = vec![cid.clone().as_slice().to_vec()];
    let evm_addr = get_eth_addr(Some(cid), Some(derivation_path), KEY_NAME.to_string()).await
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
    start_block: u64,
    max_waiting_time: u64,
    max_cache_msg: u64
) -> Result<bool, String> {
    // add chain config
    // need to deploy gateway canister manually
    // provide the gateway canister principal, as the WASM size will exceed if include the gateway canister bytes
    CHAINS.with(|chains| {
        let mut chains = chains.borrow_mut();
        if !chains.contains_key(&chain_id) {
            chains.insert(
                chain_id,
                ChainState::new(
                    ChainConfig::new(
                        ChainType::Evm,
                        chain_id,
                        urls.clone(),
                        gateway_canister_addr,
                        omnic_addr.clone(),
                        start_block,
                        max_waiting_time,
                        max_cache_msg
                    )
                )
            );

            // add record
            add_record(
                ic_cdk::caller(),
                "add_chain".to_string(),
                DetailsBuilder::new()
                    .insert("chain_id", DetailValue::U64(chain_id as u64))
                    .insert("urls", DetailValue::Text(urls.join(",")))
                    .insert("gateway_addr", DetailValue::Principal(gateway_canister_addr))
                    .insert("omnic_addr", DetailValue::Text(omnic_addr))
                    .insert("start_block", DetailValue::U64(start_block))
                    .insert("max_waiting_time", DetailValue::U64(max_waiting_time))
                    .insert("startmax_cache_msg_block", DetailValue::U64(max_cache_msg))
            );
            Ok(true)
        } else {
            Err(format!("chain {} has been added!", chain_id))
        }
    })

}

#[update(name = "delete_chain", guard = "is_authorized")]
#[candid_method(update, rename = "delete_chain")]
fn delete_chain(chain_id: u32) -> Result<bool, String> {
    match
        CHAINS.with(|c| {
            let mut chains = c.borrow_mut();
            chains.remove(&chain_id)
        })
    {
        Some(_) => {
            add_record(
                ic_cdk::caller(),
                "delete_chain".to_string(),
                DetailsBuilder::new().insert("chain_id", DetailValue::U64(chain_id as u64))
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
    start_block: u64,
    max_waiting_time: u64,
    max_cache_msg: u64
) -> Result<bool, String> {
    // add chain config
    CHAINS.with(|chains| {
        let mut chains = chains.borrow_mut();
        if chains.contains_key(&chain_id) {
            chains.insert(
                chain_id,
                ChainState::new(
                    ChainConfig::new(
                        ChainType::Evm,
                        chain_id,
                        urls.clone(),
                        gateway_canister_addr,
                        omnic_addr.clone(),
                        start_block,
                        max_waiting_time,
                        max_cache_msg
                    )
                )
            );
            add_record(
                ic_cdk::caller(),
                "update_chain".to_string(),
                DetailsBuilder::new()
                    .insert("chain_id", DetailValue::U64(chain_id as u64))
                    .insert("urls", DetailValue::Text(urls.join(",")))
                    .insert("gateway_addr", DetailValue::Principal(gateway_canister_addr))
                    .insert("omnic_addr", DetailValue::Text(omnic_addr))
                    .insert("start_block", DetailValue::U64(start_block))
                    .insert("max_waiting_time", DetailValue::U64(max_waiting_time))
                    .insert("startmax_cache_msg_block", DetailValue::U64(max_cache_msg))
            );
            Ok(true)
        } else {
            Err("chain not exists, please add chain first".into())
        }
    })
}

#[query(name = "get_chains")]
#[candid_method(query, rename = "get_chains")]
fn get_chains() -> Result<Vec<ChainState>, String> {
    // add chain config
    CHAINS.with(|chains| {
        let chains = chains.borrow();
        Ok(
            chains
                .clone()
                .into_iter()
                .map(|(_id, c)| c)
                .collect()
        )
    })
}

#[update(name = "get_tx_count")]
#[candid_method(update, rename = "get_tx_count")]
async fn get_tx_count(chain_id: u32, addr: String) -> Result<u64, String> {
    // check cycles
    let available = ic_cdk::api::call::msg_cycles_available();
    let need_cycles = 10u64 * CYCLES_PER_BYTE;
    if available < need_cycles {
        return Err(
            format!("Insufficient cycles: require {} cycles. Received {}.", need_cycles, available)
        );
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
        ChainType::Evm => {}
        _ => {
            return Err("chain type not supported yet".into());
        }
    }

    let client = EVMChainClient::new(rpc_url, omnic_addr, MAX_RESP_BYTES, CYCLES_PER_CALL).map_err(
        |e| format!("init client failed: {:?}", e)
    )?;

    client.get_tx_count(addr).await.map_err(|e| format!("{:?}", e))
}

#[update(name = "get_gas_price")]
#[candid_method(update, rename = "get_gas_price")]
async fn get_gas_price(chain_id: u32) -> Result<u64, String> {
    // check cycles
    let available = ic_cdk::api::call::msg_cycles_available();
    let need_cycles = 10u64 * CYCLES_PER_BYTE;
    if available < need_cycles {
        return Err(
            format!("Insufficient cycles: require {} cycles. Received {}.", need_cycles, available)
        );
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
        ChainType::Evm => {}
        _ => {
            return Err("chain type not supported yet".into());
        }
    }

    let client = EVMChainClient::new(rpc_url, omnic_addr, MAX_RESP_BYTES, CYCLES_PER_CALL).map_err(
        |e| format!("init client failed: {:?}", e)
    )?;

    client.get_gas_price().await.map_err(|e| format!("{:?}", e))
}

// application canister call this method to send tx to destination chain
#[update(name = "send_raw_tx")]
#[candid_method(update, rename = "send_raw_tx")]
async fn send_raw_tx(dst_chain: u32, raw_tx: Vec<u8>) -> Result<Vec<u8>, String> {
    // check cycles
    let available = ic_cdk::api::call::msg_cycles_available();
    let need_cycles = (raw_tx.len() as u64) * CYCLES_PER_BYTE;
    if available < need_cycles {
        return Err(
            format!("Insufficient cycles: require {} cycles. Received {}.", need_cycles, available)
        );
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
        ChainType::Evm => {}
        _ => {
            return Err("chain type not supported yet".into());
        }
    }

    let client = EVMChainClient::new(rpc_url, omnic_addr, MAX_RESP_BYTES, CYCLES_PER_CALL).map_err(
        |e| format!("init client failed: {:?}", e)
    )?;

    // client.send_raw_tx will always end up with error because the same tx will be submitted multiple times
    // by the node in the subnet, first submission response ok, the rest will response error,
    // so we should ignore return value of send_raw_tx, then query by the txhash to make sure the tx is correctly sent
    client.send_raw_tx(raw_tx).await.map_err(|e| format!("{:?}", e))
    // TODO: fetch via client.get_tx_by_hash to make sure the tx is included
}

// update chain settings
// clear existing messages cache immediately
#[update(name = "trigger_clear_cache", guard = "is_authorized")]
#[candid_method(update, rename = "trigger_clear_cache")]
async fn trigger_clear_cache(dst_chains: Vec<u32>) -> Result<bool, String> {
    for dst_chain in dst_chains {
        let (msgs, msg_len) = CACHE.with(|cache| {
            let c = cache.borrow();
            (c.encode_msgs(dst_chain), c.get_cache_messages_len(dst_chain))
        });

        let (caller, omnic_addr, rpc) = CHAINS.with(|chains| {
            let chains = chains.borrow();
            let c = chains.get(&dst_chain).expect("chain not found");
            (c.canister_addr.clone(), c.config.omnic_addr.clone(), c.config.rpc_urls[0].clone())
        });
        if caller == "" || omnic_addr == "" {
            return Err("caller address is empty".into());
        }
        match call_to_chain(caller, omnic_addr, rpc, dst_chain, msgs).await {
            Ok(txhash) => {
                //clear cache
                CACHE.with(move |cache| cache.borrow_mut().clean_messages(dst_chain));

                //add record
                add_record(
                    ic_cdk::caller(),
                    "process_message_batch".to_string(),
                    DetailsBuilder::new()
                        .insert("origin", DetailValue::U64(0u64))
                        .insert("destination", DetailValue::U64(dst_chain as u64))
                        .insert("message_batch", DetailValue::U64(msg_len))
                        .insert("result", DetailValue::Text(txhash.to_string()))
                );
            }
            Err(err) => {
                return Err(err);
            }
        }
    }
    Ok(true)
}

// call by application
// cache message and send them as a batch when it reaches the maximum capacity of the cache or
// the limited time
#[update(name = "send_message")]
#[candid_method(update, rename = "send_message")]
async fn send_message(msg_type: u8, dst_chain: u32, recipient: [u8;32], payload: Vec<u8>) -> Result<bool, String> {
    let t = MessageType::from_u8(msg_type)?;
    // check cycles
    let available = ic_cdk::api::call::msg_cycles_available();
    let need_cycles = (payload.len() as u64) * CYCLES_PER_BYTE;
    if available < need_cycles {
        return Err(
            format!("Insufficient cycles: require {} cycles. Received {}.", need_cycles, available)
        );
    }
    // accept cycles
    let _accepted = ic_cdk::api::call::msg_cycles_accept(need_cycles);

    let api_caller = ic_cdk::api::caller();
    let out_nonce = get_out_nonce(&dst_chain, &api_caller) + 1;
    add_log(format!("send message caller:{:?}, out bound nonce: {}", Principal::to_text(&api_caller), out_nonce));

    // padding caller to 32 bytes
    let sender = api_caller.clone().as_slice().to_owned();
    let mut padding_sender = vec![];
    padding_sender.resize(32 - sender.len(), 0);
    padding_sender.extend_from_slice(&sender);
    let msg = Message {
        t,
        origin: 0u32,
        sender: H256::from_slice(&padding_sender),
        nonce: out_nonce,
        destination: dst_chain,
        recipient: H256::from_slice(&recipient),
        body: payload,
    };

    let (max_waiting_time, max_cache_msg_cap) = _get_cache_meta(&dst_chain);

    let (msgs, msg_len, send_flag) = CACHE.with(move |cache| {
        let mut c = cache.borrow_mut();
        let front_cache_msg_ts = c.get_front_msg_ts(dst_chain); // the first msg timestamp
        let current_cache_msg_cap = c.get_cache_messages_len(dst_chain); // current message capacity
        ic_cdk::println!("first msg timestamp: {}, cap: {}", front_cache_msg_ts, current_cache_msg_cap);
        c.insert_message(dst_chain, msg);
        if
            front_cache_msg_ts + max_waiting_time > get_time() ||
            current_cache_msg_cap >= max_cache_msg_cap - 1u64
        {
            // send out
            (c.encode_msgs(dst_chain), current_cache_msg_cap + 1, true)
        } else {
            (vec![], 0u64, false)
        }
    });

    add_record(
        api_caller,
        "send_message".to_string(),
        DetailsBuilder::new()
            .insert("nonce", DetailValue::U64(out_nonce))
            .insert("destination", DetailValue::U64(dst_chain as u64))
            .insert("recipient", DetailValue::Text(hex::encode(&recipient)))
    );
    // out nonce increment
    inc_out_nonce(dst_chain, api_caller.clone());

    if send_flag {
        let (caller, omnic_addr, rpc) = CHAINS.with(|chains| {
            let chains = chains.borrow();
            let c = chains.get(&dst_chain).expect("chain not found");
            (c.canister_addr.clone(), c.config.omnic_addr.clone(), c.config.rpc_urls[0].clone())
        });
        if caller == "" || omnic_addr == "" {
            return Err("caller address is empty".into());
        }
        ic_cdk::println!("send msg {:?} to chain {}", msgs, dst_chain);
        match call_to_chain(caller, omnic_addr, rpc, dst_chain, msgs).await {
            Ok(txhash) => {
                //clear cache
                CACHE.with(move |cache| cache.borrow_mut().clean_messages(dst_chain));

                //add record
                add_record(
                    api_caller,
                    "process_message_batch".to_string(),
                    DetailsBuilder::new()
                        .insert("origin", DetailValue::U64(0u64))
                        .insert("destination", DetailValue::U64(dst_chain as u64))
                        .insert("message_batch", DetailValue::U64(msg_len))
                        .insert("result", DetailValue::Text(txhash.to_string()))
                );
                return Ok(true);
            }
            Err(err) => {
                return Err(err);
            }
        }
    }

    Ok(true)
}

// only gateway canister call
#[update(name = "process_message", guard = "is_authorized")]
#[candid_method(update, rename = "process_message")]
async fn process_message(messages: Vec<MessageStable>) -> Result<Vec<(String, u64)>, String> {
    // now, proxy handle message one by one and add batch processing in the future
    let caller = ic_cdk::caller();
    let mut rets: Vec<(String, u64)> = vec![];
    for m in messages {
        if m.nonce != get_in_nonce(&m.origin, H256::from(m.sender).to_string().as_ref()) + 1 {
            add_log(format!("expected nonce != current nonce: {} != {}", m.origin, m.nonce));
            return Err(format!("expected nonce != current nonce: {} != {}", m.origin, m.nonce));
        }
        let res = if m.destination == 0u32 {
            // take last 10 bytes
            let recipient = Principal::from_slice(&m.recipient[22..]);
            add_log(format!("recipient: {:?}", Principal::to_text(&recipient)));
            ic_cdk::println!("dispatch message to destinated canister!");
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

            call_to_chain(
                caller,
                omnic_addr,
                rpc,
                m.destination,
                vec![encode_body(&Message::from(m.clone()))]
            ).await
        };
        //update
        inc_in_nonce(m.origin, H256::from(m.sender).to_string());
        add_record(
            caller,
            "process_message".to_string(),
            DetailsBuilder::new()
                .insert("origin", DetailValue::U64(m.origin as u64))
                .insert("send", DetailValue::Text(H256::from(m.sender).to_string()))
                .insert("nonce", DetailValue::U64(m.nonce as u64))
                .insert("destination", DetailValue::U64(m.destination as u64))
                .insert("recipient", DetailValue::Text(H256::from(m.recipient).to_string()))
                .insert(
                    "result",
                    DetailValue::Text(
                        res.clone().map_or_else(
                            |e| e,
                            |o| o
                        )
                    )
                )
        );
        let r = res.map(|o| (o, ic_cdk::api::time()));
        if r.is_ok() {
            rets.push(r.unwrap());
        }
    }
    Ok(rets)
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
            Some((s, e)) => { (s, e) }
            None => {
                // range not set, default to last 50 records
                let size = records.size(operation.clone());
                if size < 50 {
                    (0, size)
                } else {
                    (size - 50, size)
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
    let chains = CHAINS.with(|c| { c.replace(HashMap::default()) });
    let owners = OWNERS.with(|o| { o.replace(HashSet::default()) });
    let records = RECORDS.with(|r| { r.replace(RecordDB::new()) });
    ic_cdk::storage::stable_save((chains, owners, records)).expect("pre upgrade error");
}

#[post_upgrade]
fn post_upgrade() {
    let (chains, owners, records): (
        HashMap<u32, ChainState>,
        HashSet<Principal>,
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

fn _get_cache_meta(chain: &u32) -> (u64, u64) {
    CHAINS.with(|chains| {
        let chains = chains.borrow();
        chains.get(chain).map_or((0u64, 0u64), |c| c.config.get_cache_info())
    })
}

fn is_authorized() -> Result<(), String> {
    let user = ic_cdk::api::caller();
    OWNERS.with(|owner| {
        if !owner.borrow().contains(&user) { Err("unauthorized!".into()) } else { Ok(()) }
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
        records.append(caller, get_time(), op, details_builder.build());
    });
}

fn get_out_nonce(dst_chain: &u32, canister: &Principal) -> u64 {
    RECORDS.with(|r| r.borrow().get_out_nonce(dst_chain, canister))
}

fn get_in_nonce(dst_chain: &u32, sender: &str) -> u64 {
    RECORDS.with(|r| r.borrow().get_in_nonce(dst_chain, sender))
}
fn inc_out_nonce(dst_chain: u32, canister: Principal) {
    RECORDS.with(|r| r.borrow_mut().inc_out_nonce(dst_chain, canister));
}
fn inc_in_nonce(dst_chain: u32, sender: String) {
    RECORDS.with(|r| r.borrow_mut().inc_in_nonce(dst_chain, sender));
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