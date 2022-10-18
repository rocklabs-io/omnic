use ic_cdk::export::candid::{candid_method, Deserialize, CandidType, Nat, Principal};
use ic_cdk_macros::{query, update, pre_upgrade, post_upgrade, init};
use ic_cdk::api::call::CallResult;
use ic_cdk::api::call::call_with_payment;

use ic_web3::transports::ICHttp;
use ic_web3::Web3;
use ic_web3::ic::{get_eth_addr, KeyInfo};
use ic_web3::{
    contract::{Contract, Options},
    ethabi::ethereum_types::{U64, U256},
    types::{Address, H256},
};
use omnic_bridge::pool::Pool;
use omnic_bridge::router::{Router, BridgeRouters};
use omnic_bridge::token::Token;
use omnic_bridge::utils::*;
use omnic_bridge::dip20::*;
use std::cell::RefCell;
use std::str::FromStr;
use std::collections::HashSet;
use std::result::Result;

ic_cron::implement_cron!();

/*
enum OperationTypes {
        Invalid, // 0
        AddLiquidity, // 1
        Swap, // 2
        RemoveLiquidity, // 3
        CreatePool // 4
    }
*/
const OPERATION_ADD_LIQUIDITY: u8 = 1;
const OPERATION_SWAP: u8 = 2;
const OPERATION_REMOVE_LIQUIDITY: u8 = 3;
const OPERATION_CREATE_POOL: u8 = 4;

const KEY_NAME: &str = "test_key_1";
const BRIDGE_ABI: &[u8] = include_bytes!("./bridge.json");

#[derive(CandidType, Deserialize, Debug, PartialEq)]
struct State {
    omnic: Principal, // omnic proxy canister
    owners: HashSet<Principal>,
    bridge_canister_addr: String, // evm address of this canister
}

impl State {
    pub fn new() -> Self {
        State {
            omnic: Principal::from_text("y3lks-laaaa-aaaam-aat7q-cai").unwrap(),
            owners: HashSet::new(),
            bridge_canister_addr: "".into(),
        }
    }

    pub fn set_omnic(&mut self, omnic_proxy: Principal) {
        self.omnic = omnic_proxy;
    }

    pub fn is_omnic(&self, caller: Principal) -> bool {
        self.omnic == caller
    }

    pub fn set_bridge_canister_addr(&mut self, addr: String) {
        self.bridge_canister_addr = addr;
    }

    pub fn get_bridge_canister_addr(&self) -> String {
        self.bridge_canister_addr.clone()
    }

    pub fn is_owner(&self, user: Principal) -> bool {
        self.owners.contains(&user)
    }

    pub fn add_owner(&mut self, user: Principal) {
        self.owners.insert(user);
    }

    pub fn remove_owner(&mut self, owner: Principal) {
        self.owners.remove(&owner);
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::new());
    static ROUTERS: RefCell<BridgeRouters> = RefCell::new(BridgeRouters::new());
}

fn is_authorized() -> Result<(), String> {
    let user = ic_cdk::api::caller();
    STATE.with(|info| {
        let info = info.borrow();
        if !info.is_owner(user) {
            Err("unauthorized!".into())
        } else {
            Ok(())
        }
    })
}

#[init]
#[candid_method(init)]
fn init() {
    let caller = ic_cdk::api::caller();
    STATE.with(|info| {
        let mut info = info.borrow_mut();
        info.add_owner(caller);
    });
}

#[update(guard = "is_authorized")]
#[candid_method(update, rename = "set_omnic")]
async fn set_omnic(omnic: Principal) -> Result<bool, String> {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.set_omnic(omnic);
    });
    Ok(true)
}

#[update(guard = "is_authorized")]
#[candid_method(update, rename = "add_owner")]
async fn add_owner(user: Principal) -> Result<bool, String> {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.add_owner(user);
    });
    Ok(true)
}

#[update(guard = "is_authorized")]
#[candid_method(update, rename = "remove_owner")]
async fn remove_owner(user: Principal) -> Result<bool, String> {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.remove_owner(user);
    });
    Ok(true)
}

// add supported chain, add to BRIDGE state
// ic: chain_id = 0, bridge_addr = ""
// goerli: chain_id = 5, bridge_addr = "xxxx"
#[update(guard = "is_authorized")]
#[candid_method(update, rename = "add_chain")]
fn add_chain(chain_id: u32, bridge_addr: String) -> Result<bool, String> {
    ROUTERS.with(|r| {
        let mut routers = r.borrow_mut();
        if routers.chain_exists(chain_id) {
            return Err("chain exists!".into());
        }
        routers.add_chain(chain_id, bridge_addr);
        Ok(true)
    })
}

// add wrapper token pool to chain ic
#[update(guard = "is_authorized")]
#[candid_method(update, rename = "create_pool")]
async fn create_pool(token_id: String, shared_decimals: u8) -> Result<bool, String> {
    let chain_id = 0u32;
    let token_pid = Principal::from_text(&token_id).unwrap();
    // get token metadata
    let res: CallResult<(Metadata, )> = ic_cdk::call(
        token_pid,
        "getMetadata",
        (),
    ).await;
    let metadata = match res {
        Ok((v, )) => {
            v
        }
        Err((_code, msg)) => {
            return Err(msg);
        }
    };
    let local_decimals = metadata.decimals;
    let token = Token::new(metadata.name, metadata.symbol, metadata.decimals, token_id.clone());

    ROUTERS.with(|r| {
        let routers = r.borrow();
        if routers.pool_exists(chain_id, &token_id) {
            return Err("pool exists!".into());
        }
        let pool_id = routers.pool_count(chain_id);
        routers.create_pool(
            chain_id,
            pool_id,
            token_id, // pool address, just a placeholder
            shared_decimals,
            local_decimals,
            token,
        );
        Ok(true)
    })
}

// calc bridge canister's evm address and store to state, only owners can call
#[update(name = "set_canister_addr")]
#[candid_method(update, rename = "set_canister_addr")]
async fn set_canister_addr() -> Result<String, String> {
    let cid = ic_cdk::id();
    let derivation_path = vec![cid.clone().as_slice().to_vec()];
    let evm_addr = get_eth_addr(Some(cid), Some(derivation_path), KEY_NAME.to_string())
        .await
        .map(|v| hex::encode(v))
        .map_err(|e| format!("calc evm address failed: {:?}", e))?;
    STATE.with(|s| s.borrow_mut().set_bridge_canister_addr(evm_addr.clone()));
    Ok(evm_addr)
}

#[update(guard = "is_authorized")]
#[candid_method(update, rename = "delete_pool")]
async fn delete_pool(chain_id: u32, pool_id: u32) -> Result<bool, String> {
    ROUTERS.with(|r| {
        let mut routers = r.borrow_mut();
        routers.remove_pool(chain_id, pool_id);
        Ok(true)
    })
}

#[update(guard = "is_authorized")]
#[candid_method(update, rename = "add_liquidity")]
async fn add_liquidity(chain_id: u32, pool_id: u32, amount: u64) -> Result<bool, String> {
    ROUTERS.with(|r| {
        let routers = r.borrow_mut();
        routers.add_liquidity(chain_id, pool_id, amount as u128);
        Ok(true)
    })
}

#[update(guard = "is_authorized")]
#[candid_method(update, rename = "remove_liquidity")]
async fn remove_liquidity(chain_id: u32, pool_id: u32, amount: u64) -> Result<bool, String> {
    ROUTERS.with(|r| {
        let routers = r.borrow_mut();
        routers.remove_liquidity(chain_id, pool_id, amount as u128);
        Ok(true)
    })
}

// check if there's enough liquidity for a swap
#[query(name = "check_swap")]
#[candid_method(query, rename = "check_swap")]
fn check_swap(dst_chain: u32, dst_pool: u32, amount: u64) -> Result<bool, String> {
    ROUTERS.with(|r| {
        let routers = r.borrow();
        Ok(routers.check_swap(dst_chain, dst_pool, amount.into()))
    })
}

#[query(name = "get_router")]
#[candid_method(query, rename = "get_router")]
fn get_router(chain_id: u32) -> Result<Router, String> {
    ROUTERS.with(|r| {
        Ok(r.borrow().get_router(chain_id))
    })
}

#[query(name = "get_routers")]
#[candid_method(query, rename = "get_routers")]
fn get_routers() -> Result<BridgeRouters, String> {
    ROUTERS.with(|r| {
        Ok(r.borrow().clone())
    })
}

// chain id -> token address -> Pool
#[query(name = "pool_by_token_address")]
#[candid_method(query, rename = "pool_by_token_address")]
fn pool_by_token_address(chain_id: u32, token_addr: String) -> Result<Pool, String> {
    ROUTERS.with(|r| {
        let routers = r.borrow();
        if !routers.pool_exists(chain_id, &token_addr) {
            return Err("pool for this token not exist!".into());
        }
        Ok(routers.pool_by_token_address(chain_id, &token_addr))
    })
}

// handle message, only omnic proxy canister can call
#[update(name = "handle_message")]
#[candid_method(update, rename = "handle_message")]
async fn handle_message(src_chain: u32, sender: Vec<u8>, _nonce: u32, payload: Vec<u8>) -> Result<bool, String> {
    // only omnic proxy canister can call
    let caller: Principal = ic_cdk::api::caller();
    STATE.with(|info| {
        let info = info.borrow();
        if !info.is_omnic(caller) {
            return Err("!omnic_proxy".to_string());
        } else {
            Ok(())
        }
    })?;
    // sender on src chain must be corresponding bridge contract
    // ROUTERS.with(|r| {
    //     let r = r.borrow();
    //     let bridge_addr = {
    //         let s = r.bridge_addr(src_chain).trim_start_matches("0x").to_string();
    //         s.to_lowercase()
    //     };
    //     let sender_str = {
    //         let temp = hex::encode(&sender);
    //         temp.trim_start_matches("0x").to_string()
    //     };
    //     if sender_str != bridge_addr {
    //         return Err("msg sender is not bridge contract!".to_string());
    //     }
    //     Ok(())
    // })?;
    
    let operation_type = get_operation_type(&payload)?;

    if operation_type == OPERATION_ADD_LIQUIDITY {
        _handle_operation_add_liquidity(src_chain, sender.clone(), &payload)
    } else if operation_type == OPERATION_REMOVE_LIQUIDITY {
        _handle_operation_remove_liquidity(src_chain, sender.clone(), &payload)
    } else if operation_type == OPERATION_SWAP {
        _handle_operation_swap(src_chain, &payload).await
    } else if operation_type == OPERATION_CREATE_POOL {
        _handle_operation_create_pool(src_chain, &payload)
    } else {
        Err("unsupported!".to_string())
    }
}

fn _handle_operation_add_liquidity(src_chain: u32, _sender: Vec<u8>, payload: &[u8]) -> Result<bool, String> {
    let (
        _src_chain_id, 
        src_pool_id, 
        amount_ld
    ) = decode_operation_liquidity(payload)?;

    ROUTERS.with(|routers| {
        let routers = routers.borrow();
        routers.add_liquidity(src_chain, src_pool_id, amount_ld);
    });
    Ok(true)
}

fn _handle_operation_remove_liquidity(src_chain: u32, _sender: Vec<u8>, payload: &[u8]) -> Result<bool, String> {
    let (
        _src_chain_id, 
        src_pool_id, 
        amount_ld
    ) = decode_operation_liquidity(payload)?;

    ROUTERS.with(|routers| {
        let routers = routers.borrow();
        routers.remove_liquidity(src_chain, src_pool_id, amount_ld);
    });
    Ok(true)
}

async fn _handle_operation_swap(_src_chain: u32, payload: &[u8]) -> Result<bool, String> {
    let (
        src_chain_id, 
        src_pool_id, 
        dst_chain_id, 
        dst_pool_id, 
        amount_sd, 
        recipient
    ) = decode_operation_swap(payload)?;

    // if dst chain_id == 0 means mint/lock mode for evm <=> ic
    // else means swap between evms
    if dst_chain_id == 0 {
        assert_ne!(src_chain_id, dst_chain_id, "src_chain_id == dst_chain_id");
        // get wrapper token cansider address
        let token = ROUTERS.with(|routers| {
            let routers = routers.borrow();
            routers.pool_token(dst_chain_id, dst_pool_id)
        });

        // mint wrapper token on IC
        let amount_ld: u128 = ROUTERS.with(|routers| {
            let routers = routers.borrow();
            routers.amount_ld(dst_chain_id, dst_pool_id, amount_sd)
        });
        let amount_ic = Nat::from(amount_ld);
        
        // TODO: recipient pid process
        // let recipient_str = String::from_utf8(recipient.clone()).unwrap();
        // let properly_trimmed_string = recipient_str.trim_matches(|c: char| c.is_whitespace() || c=='\0');
                        
        // the length of slice only be 0, 4, 29
        // let recipient_addr: Principal = Principal::from_slice(properly_trimmed_string.as_bytes());
        let recipient_addr: Principal = Principal::from_slice(&recipient[3..]);

        // DIP20
        let token_canister_id: Principal = Principal::from_text(&token.address).unwrap();
        let mint_res: CallResult<(TxReceipt, )> = ic_cdk::call(
            token_canister_id,
            "mint",
            (recipient_addr, amount_ic),
        ).await;
        match mint_res {
            Ok((res, )) => {
                match res {
                    Ok(_) => {}
                    Err(err) => {
                        return Err(format!("mint error: {:?}", err));
                    }
                }
            }
            Err((_code, msg)) => {
                return Err(msg);
            }
        }
        // update liquidity info
        ROUTERS.with(|routers| {
            let routers = routers.borrow();
            let amount_ld = routers.amount_ld(src_chain_id, src_pool_id, amount_sd);
            routers.add_liquidity(src_chain_id, src_pool_id, amount_ld);
        });
        Ok(true)
    } else {
        let (dst_pool, dst_bridge_addr) = ROUTERS.with(|routers| {
            let routers = routers.borrow();
            if !routers.check_swap(dst_chain_id, dst_pool_id, amount_sd) {
                return Err("Not enough liquidity on destination chain for this swap!");
            }
            let router = routers.get_router(dst_chain_id);
            Ok((routers.pool_by_id(dst_chain_id, dst_pool_id), router.bridge_addr()))
        })?;
        // send tx to destination chain
        let amount_ld = dst_pool.amount_ld(amount_sd);
        let _txhash = handle_swap(dst_chain_id, dst_bridge_addr, dst_pool_id, amount_ld, recipient).await?;
        // update state
        ROUTERS.with(|routers| {
            let routers = routers.borrow();
            routers.swap(src_chain_id, src_pool_id, dst_chain_id, dst_pool_id, amount_sd);
        });
        Ok(true)
    }
}

fn _handle_operation_create_pool(src_chain: u32, payload: &[u8]) -> Result<bool, String> {
    let (
        src_pool_id, 
        pool_addr, 
        token_addr, 
        shared_decimals, 
        local_decimals, 
        token_name, 
        token_symbol
    ) = decode_operation_create_pool(payload)?;
    
    let token = Token::new(
        token_name,
        token_symbol,
        local_decimals,
        token_addr,
    );
    ROUTERS.with(|routers| {
        let routers = routers.borrow();
        routers.create_pool(src_chain, src_pool_id, pool_addr, shared_decimals, local_decimals, token);
    });
    Ok(true)
}

async fn handle_swap(dst_chain: u32, dst_bridge: String, dst_pool: u32, amount_ld: u128, to: Vec<u8>) -> Result<String, String> {
    // ecdsa key info
    let derivation_path = vec![ic_cdk::id().as_slice().to_vec()];
    let key_info = KeyInfo{ derivation_path: derivation_path, key_name: KEY_NAME.to_string() };

    let w3 = match ICHttp::new("".into(), None, None) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { return Err(e.to_string()) },
    };
    let contract_address = Address::from_str(&dst_bridge).unwrap();
    let contract = Contract::from_json(
        w3.eth(),
        contract_address,
        BRIDGE_ABI
    ).map_err(|e| format!("init contract failed: {}", e))?;

    let c_addr = STATE.with(|state| state.borrow().get_bridge_canister_addr());
    // add nonce to options
    let tx_count = get_nonce(dst_chain, c_addr.clone())
        .await
        .map_err(|e| format!("get tx count error: {}", e))?;
    // get gas_price
    let gas_price = get_gas_price(dst_chain)
        .await
        .map_err(|e| format!("get gas_price error: {}", e))?;
    // legacy transaction type is still ok
    let options = Options::with(|op| { 
        op.nonce = Some(tx_count);
        op.gas_price = Some(gas_price);
        op.transaction_type = Some(U64::from(2)) //EIP1559_TX_ID
    });
    // params: u256, u256, bytes32
    let mut temp = vec![0;12];
    let mut to_addr = to.clone();
    temp.append(&mut to_addr);
    let to_addr = H256::from_slice(&temp);

    let value = U256::from(amount_ld);

    let signed = contract
        .sign("handleSwap", (dst_pool, value, to_addr,), options, key_info, dst_chain as u64)
        .await
        .map_err(|e| format!("sign handleSwap failed: {}", e))?;

    let raw_tx: Vec<u8> = signed.raw_transaction.0; 
    let proxy_canister: Principal = STATE.with(|s| s.borrow().omnic.clone());
    let cycles = raw_tx.len() as u64 * 10000u64;
    let call_res: CallResult<(Result<Vec<u8>, String>, )> = call_with_payment(
        proxy_canister,
        "send_raw_tx",
        (dst_chain, raw_tx),
        cycles,
    ).await;
    match call_res {
        Ok((res, )) => {
            let mut msg: String = match res {
                Ok(_hash) => {"".into()},
                Err(mut e) => {
                    e.push_str("\t txhash:");
                    e
                },
            };
            msg.push_str(&hex::encode(signed.transaction_hash.as_bytes()));
            Ok(msg)
        }
        Err((_code, msg)) => {
            return Err(msg);
        }
    }
}

// swap from ic to evm
#[update(name = "swap")]
#[candid_method(update, rename = "swap")]
async fn swap(pool_id: u32, dst_chain: u32, dst_pool: u32, to: String, amount_ld: u64) -> Result<String, String> {
    let caller = ic_cdk::caller();
    let pool = ROUTERS.with(|routers| {
        let routers = routers.borrow();
        routers.pool_by_id(0, pool_id)
    });
    // DIP20
    let token_canister = Principal::from_text(&pool.token().address).unwrap();
    let amount = Nat::from(amount_ld);
    let burn_res: CallResult<(TxReceipt, )> = ic_cdk::call(
        token_canister,
        "burnFrom",
        (caller, amount, ),
    ).await;
    match burn_res {
        Ok((res, )) => {
            match res {
                Ok(_) => {}
                Err(err) => {
                    return Err(format!("burnFrom error: {:?}", err));
                }
            }
        }
        Err((_code, msg)) => {
            return Err(msg);
        }
    }

    let amount_sd = pool.amount_sd(amount_ld.into());
    let amount_evm_ld = ROUTERS.with(|routers| {
        let routers = routers.borrow();
        routers.amount_ld(dst_chain, dst_pool, amount_sd)
    });
    let dst_bridge_addr = ROUTERS.with(|routers| {
        let routers = routers.borrow();
        let router = routers.get_router(dst_chain);
        router.bridge_addr()
    });
    // if to address starts with 0x, trim 0x
    let to_trim =  (&to).trim();
    let to = if to_trim.starts_with("0x") {
        to_trim.strip_prefix("0x").unwrap().to_string()
    } else {
        to_trim.to_string()
    };
    let to = hex::decode(&to).expect("to address decode error");
    let to = to.to_vec();
    handle_swap(dst_chain, dst_bridge_addr, dst_pool, amount_evm_ld, to).await
}


#[pre_upgrade]
fn pre_upgrade() {
    let state = STATE.with(|c| {
        c.replace(State::new())
    });
    let routers = ROUTERS.with(|s| {
        s.replace(BridgeRouters::new())
    });
    ic_cdk::storage::stable_save((state, routers)).expect("pre upgrade error");
}

#[post_upgrade]
fn post_upgrade() {
    let (state, 
        routers
    ): (State, 
        BridgeRouters
    ) = ic_cdk::storage::stable_restore().expect("post upgrade error");
    
    STATE.with(|c| {
        c.replace(state);
    });
    ROUTERS.with(|s| {
        s.replace(routers);
    });
}

// call proxy.get_nonce() to get nonce
async fn get_nonce(chain_id: u32, addr: String) -> Result<U256, String> {
    let proxy_canister: Principal = STATE.with(|s| s.borrow().omnic.clone());
    let cycles: u64 = 100000;
    let call_res: CallResult<(Result<u64, String>, )> = call_with_payment(
        proxy_canister,
        "get_tx_count",
        (chain_id, addr,),
        cycles
    ).await;
    match call_res {
        Ok((res, )) => {
            res.map(|v| U256::from(v))
        }
        Err((_code, msg)) => {
            return Err(msg);
        }
    }
}

// call proxy.get_gas_price() to get nonce
async fn get_gas_price(chain_id: u32) -> Result<U256, String> {
    let proxy_canister: Principal = STATE.with(|s| s.borrow().omnic.clone());
    let cycles: u64 = 100000;
    let call_res: CallResult<(Result<u64, String>, )> = call_with_payment(
        proxy_canister,
        "get_gas_price",
        (chain_id,),
        cycles
    ).await;
    match call_res {
        Ok((res, )) => {
            res.map(|v| U256::from(v))
        }
        Err((_code, msg)) => {
            return Err(msg);
        }
    }
}

fn main() {
    candid::export_service!();
    std::print!("{}", __export_service());
}