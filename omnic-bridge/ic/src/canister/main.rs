use ic_cdk::export::candid::{candid_method, Deserialize, CandidType, Nat, Principal};
use ic_cdk_macros::{query, update, pre_upgrade, post_upgrade};
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
use num_bigint::BigUint;
use omnic_bridge::pool::Pool;
use omnic_bridge::router::{Router, RouterInterfaces};
use omnic_bridge::token::Token as BridgeToken;
use omnic_bridge::utils::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::str::FromStr;

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

const OWNER: &'static str = "aaaaa-aa";
const PROXY: &'static str = "y3lks-laaaa-aaaam-aat7q-cai"; // update when proxy canister deployed.

const URL: &'static str = "https://eth-goerli.g.alchemy.com/v2/0QCHDmgIEFRV48r1U1QbtOyFInib3ZAm";
const KEY_NAME: &str = "test_key_1";
const BRIDGE_ABI: &[u8] = include_bytes!("./bridge.json");


#[derive(CandidType, Deserialize, Debug, PartialEq)]
pub enum TxError {
    InsufficientBalance,
    InsufficientAllowance,
    Unauthorized,
    LedgerTrap,
    AmountTooSmall,
    BlockUsed,
    ErrorOperationStyle,
    ErrorTo,
    Other(String),
}
pub type TxReceipt = std::result::Result<Nat, TxError>;

type Result<T> = std::result::Result<T, String>;

#[derive(CandidType, Deserialize, Debug, PartialEq)]
pub type State {
    pub omnic: Principal, // omnic proxy canister
    pub owners: HashSet<Principal>;
    pub bridge_canister_addr: String; // evm address of this canister
}

impl State {
    pub fn new() -> Self {

    }

    pub fn is_authorized(&self, user: Principal) -> bool {

    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::new());
    static ROUTERS: RefCell<BridgeRouters> = RefCell::new(BridgeRouters::new());
}

#[update(name = "set_omnic")]
#[candid_method(update, rename = "set_omnic")]
async fn set_omnic() -> Result<String>

#[update(name = "add_owner")]
#[candid_method(update, rename = "add_owner")]
async fn add_owner() -> Result<String>

#[update(name = "remove_owner")]
#[candid_method(update, rename = "remove_owner")]
async fn remove_owner() -> Result<String>

fn is_authorized() -> bool {

}

// add supported chain, add to BRIDGE state
// ic: chain_id = 0, bridge_addr = ""
// goerli: chain_id = 5, bridge_addr = "xxxx"
#[update(name = "add_chain")]
#[candid_method(update, rename = "add_chain")]
fn add_chain(chain_id: u32, bridge_addr: String) -> Result<String>

// calc bridge canister's evm address and store to state, only owners can call
#[update(name = "set_canister_addr")]
#[candid_method(update, rename = "set_canister_addr")]
async fn set_canister_addr() -> Result<String> {
    let cid = ic_cdk::id();
    let derivation_path = vec![cid.clone().as_slice().to_vec()];
    let evm_addr = get_eth_addr(Some(cid), Some(derivation_path), KEY_NAME.to_string())
        .await
        .map(|v| hex::encode(v))
        .map_err(|e| format!("calc evm address failed: {:?}", e))?;
    BRIDGE_ADDR.with(|addr| {
        let mut addr = addr.borrow_mut();
        *addr = evm_addr.clone();
    });
    Ok(evm_addr)
}

// handle message, only omnic proxy canister can call
#[update(name = "handle_message")]
#[candid_method(update, rename = "handle_message")]
async fn handle_message(src_chain: u32, sender: Vec<u8>, _nonce: u32, payload: Vec<u8>) -> Result<bool> {
    let operation_type = get_operation_type(payload.clone())?;

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

fn _handle_operation_add_liquidity(src_chain: u32, sender: Vec<u8>, payload: &[u8]) -> Result<bool> {
    let (_src_chain_id, src_pool_id, amount_ld) = decode_operation_liquidity(payload)?;

    ROUTERS.with(|routers| {
        let mut routers = routers.borrow_mut();
        routers.add_liquidity(src_chain, src_pool_id, amount_ld);
    });
    Ok(true)
}

fn _handle_operation_remove_liquidity(src_chain: u32, sender: Vec<u8>, payload: &[u8]) -> Result<bool> {
    let (_src_chain_id, src_pool_id, amount) = decode_operation_liquidity(payload)?;

    ROUTERS.with(|routers| {
        let mut routers = routers.borrow_mut();
        routers.remove_liquidity(src_chain, src_pool_id, amount_ld);
    });
    Ok(true)
}

async fn _handle_operation_swap(src_chain: u32, payload: &[u8]) -> Result<bool> {
    let (src_chain_id, src_pool_id, dst_chain_id, dst_pool_id, amount_sd, recipient) = decode_operation_swap(payload)?;

    // if dst chain_id == 0 means mint/lock mode for evm <=> ic
    // else means swap between evms
    if dst_chain_id == 0 {
        // get wrapper token cansider address
        let token: String = ROUTERS.with(|routers| {
            let routers = routers.borrow();
            let token = routers.get_pool_token(dst_chain_id, dst_pool_id);
            token.address
        });

        // mint wrapper token on IC
        let amount_ld: u128 = ROUTERS.with(|routers| {
            let routers = routers.borrow();
            routers.amount_ld(dst_chain_id, dst_pool_id, amount_sd);
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
            let mut routers = routers.borrow_mut();
            let amount_ld = routers.amount_ld(src_chain_id, src_pool_id, amount_sd);
            routers.add_liquidity(src_chain_id, src_pool_id, amount_ld);
        });
        Ok(true)
    } else {
        let dst_pool = ROUTERS.with(|routers| {
            let routers = routers.borrow();
            if !routers.check_swap(src_chain_id, src_pool_id, dst_chain_id, dst_pool_id, amount_sd) {
                return Err("Not enough liquidity on destination chain for this swap!");
            }
            routers.get_pool(dst_chain_id, dst_pool_id)
        });
        // send tx to destination chain
        let amount_ld = dst_pool.amount_ld(amount_sd);
        let txhash = handle_swap(dst_chain_id, dst_pool.bridge_addr.clone(), dst_pool_id, amount_ld, recipient).await?;
        // update state
        ROUTERS.with(|routers| {
            let mut routers = routers.borrow_mut();
            routers.swap(src_chain_id, src_pool_id, dst_chain_id, dst_pool_id, amount_sd);
        });
        Ok(true)
    }
}

fn _handle_operation_create_pool(src_chain: u32, payload: &[u8]) -> Result<bool> {
    let (src_pool_id, pool_addr, token_addr, shared_decimals, local_decimals, token_name, token_symbol) = decode_operation_create_pool(payload)?;
    
    let token = Token::new(
        token_name,
        token_symbol,
        local_decimals,
        token_addr,
    );
    ROUTERS.with(|routers| {
        let mut routers = routers.borrow_mut();
        routers.create_pool(src_chain, src_pool_id, pool_addr, shared_decimals, local_decimals, token);
    });
    Ok(true)
}

async fn handle_swap(dst_chain: u32, dst_bridge: String, dst_pool: u32, amount_ld: u128, to: Vec<u8>) -> Result<String> {
    // ecdsa key info
    let derivation_path = vec![ic_cdk::id().as_slice().to_vec()];
    let key_info = KeyInfo{ derivation_path: derivation_path, key_name: KEY_NAME.to_string() };

    let w3 = match ICHttp::new(URL, None, None) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { return Err(e.to_string()) },
    };
    let contract_address = Address::from_slice(&dst_bridge);
    let contract = Contract::from_json(
        w3.eth(),
        contract_address,
        BRIDGE_ABI
    ).map_err(|e| format!("init contract failed: {}", e))?;

    let c_addr = STATE.with(|state| state.borrow().bridge_canister_addr.clone());
    // add nonce to options
    let tx_count = get_nonce(chain_id, c_addr.clone())
        .await
        .map_err(|e| format!("get tx count error: {}", e))?;
    // get gas_price
    let gas_price = get_gas_price(chain_id)
        .await
        .map_err(|e| format!("get gas_price error: {}", e))?;
    // legacy transaction type is still ok
    let options = Options::with(|op| { 
        op.nonce = Some(tx_count);
        op.gas_price = Some(gas_price);
        op.transaction_type = Some(U64::from(2)) //EIP1559_TX_ID
    });
    // params: u256, u256, bytes32
    let mut temp = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
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
    let call_res: CallResult<(Result<Vec<u8>>, )> = call_with_payment(
        proxy_canister,
        "send_raw_tx",
        (chain_id, raw_tx),
        cycles,
    ).await;
    match call_res {
        Ok((_res, )) => {
            Ok(hex::encode(signed.message_hash.as_bytes()))
        }
        Err((_code, msg)) => {
            return Err(msg);
        }
    }
}

#[update(name = "bridge_to_evm")]
#[candid_method(update, rename = "bridge_to_evm")]
async fn bridge_to_evm(wrapper_token_addr: Principal, chain_id: u32, to: String, amount: Nat) -> Result<String> {
    let caller = ic_cdk::caller();
    // DIP20
    let burn_res: CallResult<(TxReceipt, )> = ic_cdk::call(
        wrapper_token_addr,
        "burnFrom",
        (caller, amount.clone(), ),
    ).await;
    match burn_res {
        Ok((res, )) => {
            match res {
                Ok(_) => {}
                Err(err) => {
                    return Err(format!("burn error: {:?}", err));
                }
            }
        }
        Err((_code, msg)) => {
            return Err(msg);
        }
    }

    // burn wrapper token on IC
    let res: CallResult<(u8, )> = ic_cdk::call(
        wrapper_token_addr,
        "decimals",
        (),
    ).await;
    let wrapper_decimal: u8 = res.unwrap().0;

    let res: CallResult<(String, )> = ic_cdk::call(
        wrapper_token_addr,
        "symbol",
        (),
    ).await;
    let symbol: String = res.unwrap().0;

    // TODO: fix compile error
    // let amount_evm: Nat = ROUTER.with(|router| {
    //     let r = router.borrow();
    //     let pool = r.get_pool(pool_id.clone()).unwrap();
    //     let native_deciaml: u8 = 
    //         pool.get_token_by_chain_id(chain_id)
    //             .map_or(
    //                 Err(format!("no according wrapper token for {} chain {} pool", src_chain, src_pool_id.clone())),
    //                 |token| Ok(token.token_local_decimals())
    //             ).unwrap();
    //     pool.amount_ic_to_amount_evm(amount, native_deciaml, wrapper_decimal)
    // });
    let amount_evm = amount;

    let to = hex::decode(&to).expect("to address decode error");
    let to = to.to_vec();
    let amount_evm: Vec<u8> = BigUint::from(amount_evm).to_bytes_le();
    let bridge_addr: Vec<u8> = get_bridge_addr(chain_id).unwrap();
    handle_burn(chain_id, symbol, bridge_addr, to, amount_evm).await
}

#[update(name = "create_pool")]
#[candid_method(update, rename = "create_pool")]
fn create_pool(src_chain: u32, src_pool_id: Nat, symbol: String) -> Result<Nat> {
    // let caller: Principal = ic_cdk::caller();
    // let owner: Principal = Principal::from_text(OWNER).unwrap();
    // assert!(caller == owner || caller == ic_cdk::id(), "only owner or this canister can create a new pool.");

    ROUTER.with(|router| {
        let mut r = router.borrow_mut();
        if r.contain_pool_by_symbol(&symbol).unwrap() {
            let pool_id: Nat = 
                r.get_pool_id_by_symbol(&symbol)
                    .map_err(|e| format!("fail to get pool by symbol: {}", e))?;
            return Ok(pool_id);
        }
        let pool_id: Nat = r.get_pools_length();
        let tokens: BTreeMap<u32, BridgeToken<Vec<u8>>> = BTreeMap::new();
        let pool = Pool::new(pool_id.clone(), tokens);
        r.add_pool(pool)
            .map_err(|e| format!("create pool failed: {}", e))?;
        r.add_pool_id(src_chain, src_pool_id)
            .map_err(|e| format!("add pool id failed: {}", e))?;
        r.add_pool_symbol(symbol)
            .map_err(|e| format!("add pool symbol failed: {}", e))?;
        Ok(pool_id)
    })
}

#[query(name = "get_router")]
#[candid_method(query, rename = "get_router")]
fn get_router() -> Result<Router<Vec<u8>>> {
    ROUTER.with(|router| {
        let r = router.borrow();
        Ok(r.clone())
    })
}

#[query(name = "get_state")]
#[candid_method(query, rename = "get_state")]
fn get_state() -> Result<(String, Router<Vec<u8>>, WrapperTokenAddr)> {
    let addr = BRIDGE_ADDR.with(|a| a.borrow().clone());
    let router = ROUTER.with(|router| {router.borrow().clone()});
    let wrapper = WRAPPER_TOKENS.with(|w| {w.borrow().clone()});
    Ok((addr, router, wrapper))
}

#[query(name = "get_pool_id")]
#[candid_method(query, rename = "get_pool_id")]
fn get_pool_id(src_chain: u32, src_pool_id: Nat) -> Result<Nat> {
    ROUTER.with(|router| {
        let r = router.borrow();
        r.get_pool_id(src_chain, src_pool_id)
            .map_err(|e| format!("failed to get pool id: {:?}", e))
    })
}

#[query(name = "get_pool_id_by_symbol")]
#[candid_method(query, rename = "get_pool_id_by_symbol")]
fn get_pool_id_by_symbol(symbol: String) -> Result<Nat> {
    ROUTER.with(|router| {
        let r = router.borrow();
        r.get_pool_id_by_symbol(&symbol)
            .map_err(|e| format!("failed to get pool id by symbol {}: {:?}", symbol, e))
    })
}

#[update(name = "add_supported_token")]
#[candid_method(update, rename = "add_supported_token")]
fn add_supported_token(
    src_chain: u32,
    src_pool_id: Nat,
    pool_id: Nat,
    name: String,
    symbol: String,
    local_decimals: u8,
    shared_decimals: u8,
) -> Result<bool> {
    // let caller: Principal = ic_cdk::caller();
    // let owner: Principal = Principal::from_text(OWNER).unwrap();
    // assert!(caller == owner || caller == ic_cdk::id(), "only owner or this canister can add a new token.");

    ROUTER.with(|router| {
        let mut r = router.borrow_mut();
        let mut pool = r.get_pool(pool_id.clone()).map_err(|e| format!("{}", e))?;
        if pool.contain_token(src_chain) {
            return Err(format!("{} token has been added.", symbol));
        }
        let balances: BTreeMap<Vec<u8>, Nat> = BTreeMap::new();
        let token = BridgeToken::new(
            src_chain,
            src_pool_id,
            name,
            symbol,
            local_decimals,
            shared_decimals,
            balances,
        );
        pool.add_token(src_chain, token);
        r.update_pool(pool_id, pool)
            .map_err(|e| format!("update pool failed! {}", e)) //update pool
    })
}


#[pre_upgrade]
fn pre_upgrade() {
    let bridge_addr = BRIDGE_ADDR.with(|c| {
        c.replace("".to_string())
    });
    let router = ROUTER.with(|s| {
        s.replace(Router::new())
    });
    let wrapper = WRAPPER_TOKENS.with(|s| {
        s.replace(WrapperTokenAddr::new())
    });
    ic_cdk::storage::stable_save((bridge_addr, router, wrapper)).expect("pre upgrade error");
}

#[post_upgrade]
fn post_upgrade() {
    let (bridge_addr, 
        router, 
        wrapper
    ): (String, 
        Router<Vec<u8>>, 
        WrapperTokenAddr
    ) = ic_cdk::storage::stable_restore().expect("post upgrade error");
    
    BRIDGE_ADDR.with(|c| {
        c.replace(bridge_addr);
    });
    ROUTER.with(|s| {
        s.replace(router);
    });
    WRAPPER_TOKENS.with(|s| {
        s.replace(wrapper);
    });
}

// call proxy.get_nonce() to get nonce
async fn get_nonce(chain_id: u32, addr: String) -> Result<U256> {
    let proxy_canister: Principal = Principal::from_text(PROXY).unwrap();
    let cycles: u64 = 100000;
    let call_res: CallResult<(Result<u64>, )> = call_with_payment(
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
async fn get_gas_price(chain_id: u32) -> Result<U256> {
    let proxy_canister: Principal = Principal::from_text(PROXY).unwrap();
    let cycles: u64 = 100000;
    let call_res: CallResult<(Result<u64>, )> = call_with_payment(
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