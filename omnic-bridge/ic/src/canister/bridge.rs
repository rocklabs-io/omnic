use ic_cdk::export::candid::{candid_method, Deserialize, CandidType, Nat, Principal};
use ic_cdk_macros::{query, update};
use ic_cdk::api::call::CallResult;
use ic_web3::ethabi::{decode, ParamType};
use ic_web3::transports::ICHttp;
use ic_web3::Web3;
use ic_web3::ic::{get_eth_addr, KeyInfo};
use ic_web3::{
    contract::{Contract, Options},
    ethabi::ethereum_types::{U64, U256},
    types::{Address,},
};
use num_bigint::BigUint;
use omnic_bridge::pool::Pool;
use omnic_bridge::router::{Router, RouterInterfaces};
use omnic_bridge::token::Token as BridgeToken;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::convert::TryInto;

ic_cron::implement_cron!();

const OPERATION_ADD_LIQUIDITY: u8 = 1u8;
const OPERATION_REMOVE_LIQUIDITY: u8 = 2u8;
const OPERATION_SWAP: u8 = 3;
const OPERATION_CREATE_POOL: u8 = 4;

const OWNER: &'static str = "aaaaa-aa";
const PROXY: &'static str = "aaaaa-aa"; // udpate when proxy canister deployed.

const URL: &str = "https://goerli.infura.io/v3/93ca33aa55d147f08666ac82d7cc69fd";
const KEY_NAME: &str = "dfx_test_key";
const TOKEN_ABI: &[u8] = include_bytes!("./bridge.json");


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
#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct WrapperTokenAddr
{
    wrapper_tokens: BTreeMap<Nat, String>, // pool_id -> canister address
}

impl WrapperTokenAddr {
    pub fn new() -> Self {
        WrapperTokenAddr {
            wrapper_tokens: BTreeMap::new(),
        }
    }

    pub fn get_wrapper_token_addr(&self, pool_id: Nat) -> Result<String> {
        self.wrapper_tokens.get(&pool_id)
            .ok_or(format!(
                "chain id is not found: {}",
                pool_id
            ))
            .cloned()
    }

    pub fn is_wrapper_token_exist(&self, pool_id: Nat) -> bool {
        self.wrapper_tokens.contains_key(&pool_id)
    }

    pub fn add_wrapper_token_addr(&mut self, pool_id: Nat, wrapper_canister_token: String) {
        self.wrapper_tokens.entry(pool_id).or_insert(wrapper_canister_token);
    }

    pub fn remove_wrapper_token_addr(&mut self, pool_id: Nat) -> Result<String> {
        self.wrapper_tokens.remove(&pool_id)
                .ok_or(format!(
                    "pool id is not found: {}",
                    pool_id
                ))
    }
}

thread_local! {
    static ROUTER: RefCell<Router<Vec<u8>>> = RefCell::new(Router::new());
    static WRAPPER_TOKENS: RefCell<WrapperTokenAddr> = RefCell::new(WrapperTokenAddr::new());
}

#[update(name = "process_message")]
#[candid_method(update, rename = "processMessage")]
async fn process_message(src_chain: u32, sender: Vec<u8>, nonce: u32, payload: Vec<u8>) -> Result<bool> {
    let t = vec![ParamType::Uint(8)];
    let d = decode(&t, &payload).map_err(|e| format!("payload decode error: {}", e))?;
    let operation_type: u8 = d[0]
        .clone()
        .into_uint()
        .ok_or("can not convert src_chain to U256")?
        .try_into()
        .map_err(|_| format!("convert U256 to u8 failed"))?;
    if operation_type == OPERATION_ADD_LIQUIDITY {
        let types = vec![
            ParamType::Uint(8),
            ParamType::Uint(16),
            ParamType::Uint(256),
            ParamType::Uint(256),
        ];
        let d = decode(&types, &payload).map_err(|e| format!("payload decode error: {} ", e))?;
        let src_pool_id: U256 = d[2]
            .clone()
            .into_uint()
            .ok_or("can not convert src_chain to U256".to_string())?;
        let amount: U256 = d[3]
            .clone()
            .into_uint()
            .ok_or("can not convert src_chain to U256".to_string())?;

        ROUTER.with(|router| {
            let mut r = router.borrow_mut();
            let mut buffer1 = [0u8; 32];
            let mut buffer2 = [0u8; 32];
            src_pool_id.to_little_endian(&mut buffer1);
            amount.to_little_endian(&mut buffer2);
            r.add_liquidity(
                src_chain,
                Nat::from(BigUint::from_bytes_le(&buffer1)),
                sender,
                Nat::from(BigUint::from_bytes_le(&buffer2)),
            )
            .map_err(|_| format!("add liquidity failed"))
        })
    } else if operation_type == OPERATION_REMOVE_LIQUIDITY {
        let types = vec![
            ParamType::Uint(8),
            ParamType::Uint(16),
            ParamType::Uint(256),
            ParamType::Uint(256),
        ];
        let d = decode(&types, &payload).map_err(|e| format!("payload decode error: {}", e))?;
        let src_pool_id: U256 = d[2]
            .clone()
            .into_uint()
            .ok_or("can not convert src_chain to U256".to_string())?;
        let amount: U256 = d[3]
            .clone()
            .into_uint()
            .ok_or("can not convert src_chain to U256".to_string())?;

        ROUTER.with(|router| {
            let mut r = router.borrow_mut();
            let mut buffer1 = [0u8; 32];
            let mut buffer2 = [0u8; 32];
            src_pool_id.to_little_endian(&mut buffer1);
            amount.to_little_endian(&mut buffer2);
            r.remove_liquidity(
                src_chain,
                Nat::from(BigUint::from_bytes_le(&buffer1)),
                sender,
                Nat::from(BigUint::from_bytes_le(&buffer2)),
            )
            .map_err(|_| format!("remove liquidity failed"))
        })
    } else if operation_type == OPERATION_SWAP {
        let types = vec![
            ParamType::Uint(8),
            ParamType::Uint(16),
            ParamType::Uint(256),
            ParamType::Uint(16),
            ParamType::Uint(256),
            ParamType::Uint(256),
            ParamType::FixedBytes(32), 
        ];
        let d = decode(&types, &payload).map_err(|e| format!("payload decode error: {}", e))?;
        let src_chain_id: u32 = d[1]
            .clone()
            .into_uint()
            .ok_or("can not convert src_chain to U256".to_string())?
            .try_into().map_err(|_| format!("convert U256 to u32 failed"))?;
        let src_pool_id: U256 = d[2]
            .clone()
            .into_uint()
            .ok_or("can not convert src_pool_id to U256".to_string())?;
        let dst_chain_id: u32 = d[3]
            .clone()
            .into_uint()
            .ok_or("can not convert dst_chain to U256".to_string())?
            .try_into().map_err(|_| format!("convert U256 to u32 failed"))?;
        let dst_pool_id: U256 = d[4]
            .clone()
            .into_uint()
            .ok_or("can not convert dst_pool_id to U256".to_string())?;
        let amount: U256 = d[5]
            .clone()
            .into_uint()
            .ok_or("can not convert amount to U256".to_string())?;
        let recipient: Vec<u8> = d[6]
            .clone()
            .into_fixed_bytes()
            .ok_or("can not convert recipient to bytes")?;

        // if dst chain_id == 0 means mint/lock mode for evm <=> ic
        // else means swap between evms
        if dst_chain_id == 0 {
            let mut buffer1 = [0u8; 32];
            let mut buffer2 = [0u8; 32];
            let pool_id: Nat = ROUTER.with(|router| {
                let r = router.borrow();
                src_pool_id.to_little_endian(&mut buffer1);
                amount.to_little_endian(&mut buffer2);
                r.get_pool_id(src_chain_id, Nat::from(BigUint::from_bytes_le(&buffer1)))
            }).map_err(|e| format!("get pool id failed: {:?}", e))?;

            // get wrapper token cansider address
            let wrapper_token_addr: String = WRAPPER_TOKENS.with(|wrapper_tokens| {
                let w = wrapper_tokens.borrow();
                w.get_wrapper_token_addr(pool_id)
            }).map_err(|e| format!("get wrapper token address failed: {}", e))?;

            let wrapper_token_addr: Principal = Principal::from_text(&wrapper_token_addr).unwrap();
            let recipient_str = String::from_utf8(recipient.clone()).unwrap();
            let properly_trimmed_string = recipient_str.trim_matches(|c: char| c.is_whitespace() || c=='\0');
                            
            // the length of slice only be 0, 4, 29
            let recipient_addr: Principal = Principal::from_slice(properly_trimmed_string.as_bytes());

            // DIP20
            let transfer_res: CallResult<(TxReceipt, )> = ic_cdk::call(
                wrapper_token_addr,
                "mint",
                (recipient_addr, Nat::from(BigUint::from_bytes_le(&buffer2))),
            ).await;
            match transfer_res {
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
            Ok(true)
        } else {
            ROUTER.with(|router| {
                let mut r = router.borrow_mut();
                let mut buffer1 = [0u8; 32];
                let mut buffer2 = [0u8; 32];
                let mut buffer3 = [0u8; 32];
                src_pool_id.to_little_endian(&mut buffer1);
                dst_pool_id.to_little_endian(&mut buffer2);
                amount.to_little_endian(&mut buffer3);
                // udpate token ledger
                r.swap(
                    src_chain_id,
                    Nat::from(BigUint::from_bytes_le(&buffer1)),
                    dst_chain_id,
                    Nat::from(BigUint::from_bytes_le(&buffer2)),
                    Nat::from(BigUint::from_bytes_le(&buffer3)),
                )
            }).map_err(|_| format!("remove liquidity failed"))?;
    
            // call send_token method to transfer token to recipient
            let dst_bridge_addr: Vec<u8> = get_bridge_addr(dst_chain_id).unwrap();
    
            //send_token
            let mut buffer = [0u8; 32];
            amount.to_little_endian(&mut buffer);
            send_token(dst_chain_id, dst_bridge_addr, recipient, buffer.to_vec()).await // how to handle failed transfer?
        }
    } else if operation_type == OPERATION_CREATE_POOL {
        // create accroding pool to manage tokens
        let types = vec![
            ParamType::Uint(8),
            ParamType::Uint(256),
            ParamType::Uint(8), // shared_decimals
            ParamType::Uint(8), // local_decimals
            ParamType::String,
            ParamType::String, 
        ];
        let d = decode(&types, &payload).map_err(|e| format!("payload decode error: {}", e))?;

        let src_pool_id: U256 = d[1]
            .clone()
            .into_uint()
            .ok_or("can not convert src_pool_id to U256".to_string())?;
        let shared_decimal: u8 = d[2]
            .clone()
            .into_uint()
            .ok_or("can not convert shared_decimals to U256".to_string())?
            .try_into().map_err(|_| format!("convert U256 to u8 failed"))?;
        let local_decimal: u8 = d[3]
            .clone()
            .into_uint()
            .ok_or("can not convert local_decimals U256".to_string())?
            .try_into().map_err(|_| format!("convert U256 to u8 failed"))?;
        let token_name: String = d[4]
            .clone()
            .into_string()
            .ok_or("can not convert token_name to String".to_string())?;
        let token_symbol: String = d[5]
            .clone()
            .into_string()
            .ok_or("can not convert token_symbol to String".to_string())?;
        
        let mut buffer = [0u8; 32];
        src_pool_id.to_little_endian(&mut buffer);
        let pool_id : Nat = create_pool(
            src_chain, 
            Nat::from(BigUint::from_bytes_le(&buffer)), 
            token_symbol.clone()
        )?;
        add_supported_token(
            src_chain, 
            Nat::from(BigUint::from_bytes_le(&buffer)), 
            pool_id, 
            token_name, 
            token_symbol, 
            local_decimal, 
            shared_decimal
        )
    } else {
        Err("unsupported!".to_string())
    }
}


#[update(name = "burn_wrapper_token")]
#[candid_method(update, rename = "burnWrapperToken")]
async fn burn_wrapper_token(wrapper_token_addr: Principal, chain_id: u32, to: Vec<u8>, amount: Nat) -> Result<bool> {
    let caller = ic_cdk::caller();
    // DIP20
    let hole_address: Principal = Principal::from_text("aaaaa-aa").unwrap();
    let transfer_res: CallResult<(TxReceipt, )> = ic_cdk::call(
        wrapper_token_addr,
        "transferFrom",
        (caller, hole_address, amount.clone(), ),
    ).await;
    match transfer_res {
        Ok((res, )) => {
            match res {
                Ok(_) => {}
                Err(err) => {
                    return Err(format!("transferFrom error: {:?}", err));
                }
            }
        }
        Err((_code, msg)) => {
            return Err(msg);
        }
    }

    let amount: Vec<u8> = BigUint::from(amount).to_bytes_le();
    let bridge_addr: Vec<u8> = get_bridge_addr(chain_id).unwrap();
    send_token(chain_id, bridge_addr, to, amount).await
}

// call a contract, transfer some token to addr
#[update(name = "send_token")]
#[candid_method(update, rename = "send_token")]
async fn send_token(chain_id: u32, token_addr: Vec<u8>, addr: Vec<u8>, value: Vec<u8>) -> Result<bool> {
    // ecdsa key info
    let derivation_path = vec![ic_cdk::id().as_slice().to_vec()];
    let key_info = KeyInfo{ derivation_path: derivation_path, key_name: KEY_NAME.to_string() };

    let w3 = match ICHttp::new(URL, None, None) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { return Err(e.to_string()) },
    };
    let contract_address = Address::from_slice(&token_addr);
    let contract = Contract::from_json(
        w3.eth(),
        contract_address,
        TOKEN_ABI
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
    let to_addr = Address::from_slice(&addr);
    let value = U256::from_little_endian(&value);
    let signed = contract
        .sign("transfer", (to_addr, value,), options, key_info, chain_id as u64)
        .await
        .map_err(|e| format!("sign transfer failed: {}", e))?;

    let raw_tx: Vec<u8> = signed.raw_transaction.0; 
    let proxy_canister: Principal = Principal::from_text(PROXY).unwrap();
    let transfer_res: CallResult<(TxReceipt, )> = ic_cdk::call(
        proxy_canister,
        "send_raw_tx",
        (chain_id, raw_tx),
    ).await;
    match transfer_res {
        Ok((res, )) => {
            match res {
                Ok(_) => {
                    Ok(true)
                }
                Err(err) => {
                    return Err(format!("mint error: {:?}", err));
                }
            }
        }
        Err((_code, msg)) => {
            return Err(msg);
        }
    }
}

#[update(name = "create_pool")]
#[candid_method(update, rename = "createPool")]
fn create_pool(src_chain: u32, src_pool_id: Nat, symbol: String) -> Result<Nat> {
    let caller: Principal = ic_cdk::caller();
    let owner: Principal = Principal::from_text(OWNER).unwrap();
    assert!(caller == owner || caller == ic_cdk::id(), "only owner or this canister can create a new pool.");

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

#[query(name = "get_pool_id")]
#[candid_method(query, rename = "get_pool_id")]
fn get_pool_id(src_chain: u32, src_pool_id: Nat) -> Result<Nat> {

    ROUTER.with(|router| {
        let r = router.borrow();
        r.get_pool_id(src_chain, src_pool_id)
            .map_err(|e| format!("failed to get pool id: {:?}", e))
    })
}

#[update(name = "add_supported_token")]
#[candid_method(update, rename = "addSupportedToken")]
fn add_supported_token(
    src_chain: u32,
    src_pool_id: Nat,
    pool_id: Nat,
    name: String,
    symbol: String,
    local_decimals: u8,
    shared_decimals: u8,
) -> Result<bool> {
    let caller: Principal = ic_cdk::caller();
    let owner: Principal = Principal::from_text(OWNER).unwrap();
    assert!(caller == owner || caller == ic_cdk::id(), "only owner or this canister can add a new token.");

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
        r.add_pool(pool)
            .map_err(|e| format!("update pool failed! {}", e)) //update pool
    })
}

#[update(name = "add_bridge_addr")]
#[candid_method(update, rename = "addBridgeAddr")]
fn add_bridge_addr(src_chain: u32, birdge_addr: Vec<u8>) -> Result<bool> {
    let caller: Principal = ic_cdk::caller();
    let owner: Principal = Principal::from_text(OWNER).unwrap();
    assert_eq!(caller, owner);

    ROUTER.with(|router| {
        let mut r = router.borrow_mut();
        r.add_bridge_addr(src_chain, birdge_addr);
        Ok(true)
    })
}

#[update(name = "remove_bridge_addr")]
#[candid_method(update, rename = "removeBridgeAddr")]
fn remove_bridge_addr(src_chain: u32) -> Result<Vec<u8>> {
    let caller: Principal = ic_cdk::caller();
    let owner: Principal = Principal::from_text(OWNER).unwrap();
    assert_eq!(caller, owner);

    ROUTER.with(|router| {
        let mut r = router.borrow_mut();
        r.remove_bridge_addr(src_chain).map_err(|e| format!("remove bridge addr failed: {}", e))
    })
}

#[query(name = "get_bridge_addr")]
#[candid_method(query, rename = "getBridgeAddr")]
fn get_bridge_addr(chain_id: u32) -> Result<Vec<u8>> {
    ROUTER.with(|router| {
        let r = router.borrow();
        r.get_bridge_addr(chain_id)
            .map_err(|_| format!("not bridge address in {} chain", chain_id))
    })
}

#[query(name = "is_bridge_addr_exist")]
#[candid_method(query, rename = "isBridgeAddrExist")]
fn is_bridge_addr_exist(src_chain: u32) -> Result<bool> {
    ROUTER.with(|router| {
        let r = router.borrow();
        Ok(r.is_bridge_exist(src_chain))
    })
}

#[update(name = "add_wrapper_token_addr")]
#[candid_method(update, rename = "add_wrapper_token_addr")]
fn add_wrapper_token_addr(pool_id: Nat, wrapper_token_addr: String) -> Result<bool> {
    // let caller: Principal = ic_cdk::caller();
    // let owner: Principal = Principal::from_text(OWNER).unwrap();
    // assert_eq!(caller, owner);

    WRAPPER_TOKENS.with(|wrapper| {
        let mut w = wrapper.borrow_mut();
        w.add_wrapper_token_addr(pool_id, wrapper_token_addr);
        Ok(true)
    })
}

#[update(name = "remove_wrapper_token_addr")]
#[candid_method(update, rename = "remove_wrapper_token_addr")]
fn remove_wrapper_token_addr(pool_id: Nat) -> Result<String> {
    let caller: Principal = ic_cdk::caller();
    let owner: Principal = Principal::from_text(OWNER).unwrap();
    assert_eq!(caller, owner);

    WRAPPER_TOKENS.with(|wrapper| {
        let mut w = wrapper.borrow_mut();
        w.remove_wrapper_token_addr(pool_id).map_err(|e| format!("remove wrapper token addr failed: {}", e))
    })
}

#[query(name = "get_wrapper_token_addr")]
#[candid_method(query, rename = "get_wrapper_token_addr")]
fn get_wrapper_token_addr(pool_id: Nat) -> Result<String> {
    WRAPPER_TOKENS.with(|wrapper| {
        let w = wrapper.borrow();
        w.get_wrapper_token_addr(pool_id.clone())
            .map_err(|_| format!("not wrapper token address in {} chain", pool_id))
    })
}

#[query(name = "is_wrapper_token_exist")]
#[candid_method(query, rename = "is_wrapper_token_exist")]
fn is_wrapper_token_exist(pool_id: Nat) -> Result<bool> {
    WRAPPER_TOKENS.with(|wrapper| {
        let w = wrapper.borrow();
        Ok(w.is_wrapper_token_exist(pool_id))
    })
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


// the test should be executed in one thread
#[cfg(test)]
mod tests {
    use super::*;
    use ic_web3::{
        ethabi::{Token, encode, ParamType, Bytes, Uint},
        contract::tokens::{Detokenize, Tokenizable},
        types::{Address, U256, BytesArray},
    };
    use ic_cdk::api::call::CallResult;
    use ic_cdk::export::candid::{Deserialize, CandidType, Nat};
    use ic_kit::{mock_principals::{alice, bob, john}, MockContext};
    use hex_literal::hex;

    fn add_new_pool(src_chain_id: u32, src_pool_id: Nat) -> bool {
        create_pool(src_chain_id, src_pool_id.clone()).unwrap_or(false)
    }

    fn add_wrapper_token(pool_id: Nat, wrapper_token_addr: String) -> bool {
        add_wrapper_token_addr(pool_id, wrapper_token_addr).unwrap_or(false)
    }

    #[test]
    #[ignore]
    fn should_create_pool() {
        let src_chain_id: u32 = 1; //ethereum
        let src_pool_id: Nat = 0.into(); // fake usdt pool id
        let res: bool = add_new_pool(src_chain_id, src_pool_id.clone());
        assert!(res);
        let pool_id = get_pool_id(src_chain_id, src_pool_id).unwrap();
        assert_eq!(pool_id, Nat::from(0))
    }

    #[test]
    #[ignore]
    fn should_add_wrapper_token_addr() {
        let src_chain_id: u32 = 1; //ethereum
        let src_pool_id: Nat = 0.into(); // fake usdt pool id
        assert!(add_new_pool(src_chain_id, src_pool_id.clone()));
        let wrapper_token_addr: &str = "aaaaa-aa"; //wrapper usdt canister address
        let pool_id = get_pool_id(src_chain_id, src_pool_id).unwrap();
        assert!(add_wrapper_token(pool_id, wrapper_token_addr.to_string()));
    }

    #[async_std::test]
    async fn should_process_swap_message() {
        let src_chain_id: u32 = 1; //ethereum
        let src_pool_id: Nat = 0.into(); // fake usdt pool id
        assert!(add_new_pool(src_chain_id, src_pool_id.clone()));
        let wrapper_token_addr: &str = "rwlgt-iiaaa-aaaaa-aaaaa-cai"; //wrapper usdt canister address
        let pool_id = get_pool_id(src_chain_id, src_pool_id).unwrap();
        assert!(add_wrapper_token(pool_id, wrapper_token_addr.to_string()));

        let token = vec![
            3u8.into_token(), // swap
            1u16.into_token(), // src_chain_id = 1
            Token::Uint(Uint::from(0)), // src pool id = 0 (fake usdt)
            0u16.into_token(), // dst chain id = 0 (ic)
            Token::Uint(Uint::from(0)), // dst pool id = 0 (canister store usdt token from other chains)
            Token::Uint(Uint::from(10_000_000_000u128)), // token amount = 1000000000
            Token::FixedBytes("aaaaa-aa".to_string().into_bytes())
        ];
        let payload: Bytes = encode(&token);
        let sender: Vec<u8> = hex!("0000000000000000000000000000000000000000").into();

        let res: bool = process_message(src_chain_id, sender, 1, payload).await.unwrap();
        assert!(res);
    }

    #[async_std::test]
    async fn should_burn_wrapper_token() {
        let dst_chain_id: u32 = 5; //goerli ethereum
        let recipient: Vec<u8> = hex!("AAB27b150451726EC7738aa1d0A94505c8729bd1").into(); // dst chain recipient
        let amount: Nat = Nat::from(1000000);
        let wrapper_token_addr: Principal = Principal::from_text("rwlgt-iiaaa-aaaaa-aaaaa-cai").unwrap(); //wrapper usdt canister address

        let bridge_addr: Vec<u8> = hex!("AAB27b150451726EC7738aa1d0A94505c8729bd1").into(); // goerli bridge adress
        assert!(add_bridge_addr(dst_chain_id, bridge_addr).unwrap());

        let res: bool = burn_wrapper_token(wrapper_token_addr, dst_chain_id, recipient, amount).await.unwrap();
        assert!(res);
    }


}