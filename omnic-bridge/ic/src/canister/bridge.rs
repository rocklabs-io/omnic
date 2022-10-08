use ic_cdk::export::candid::{candid_method, Deserialize, CandidType, Nat, Principal};
use ic_cdk_macros::{query, update, pre_upgrade, post_upgrade};
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
    static BRIDGE_ADDR: RefCell<String> = RefCell::new("".to_string());
    static ROUTER: RefCell<Router<Vec<u8>>> = RefCell::new(Router::new());
    static WRAPPER_TOKENS: RefCell<WrapperTokenAddr> = RefCell::new(WrapperTokenAddr::new());
}

#[update(name = "set_canister_addrs")]
#[candid_method(update, rename = "set_canister_addrs")]
async fn set_canister_addrs() -> Result<String> {
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

#[update(name = "handle_message")]
#[candid_method(update, rename = "handle_message")]
async fn handle_message(src_chain: u32, sender: Vec<u8>, _nonce: u32, payload: Vec<u8>) -> Result<bool> {
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
        /*
            uint8(OperationTypes.Swap),
            uint16 _srcChainId,
            uint256 _srcPoolId,
            uint16 _dstChainId,
            uint256 _dstPoolId,
            uint256 _amountLD,
            bytes32 _to
        */
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
        let amount_evm: U256 = d[5]
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
                r.get_pool_id(src_chain_id, Nat::from(BigUint::from_bytes_le(&buffer1)))
            }).map_err(|e| format!("get pool id failed: {:?}", e))?;


            // get wrapper token cansider address
            let wrapper_token_addr: String = WRAPPER_TOKENS.with(|wrapper_tokens| {
                let w = wrapper_tokens.borrow();
                w.get_wrapper_token_addr(pool_id.clone())
            }).map_err(|e| format!("get wrapper token address failed: {}", e))?;

            // mint wrapper token on IC
            let wrapper_token_addr: Principal = Principal::from_text(&wrapper_token_addr).unwrap();
            let transfer_res: CallResult<(u8, )> = ic_cdk::call(
                wrapper_token_addr,
                "decimals",
                (),
            ).await;
            let wrapper_decimal: u8 = transfer_res.unwrap().0;
            amount_evm.to_little_endian(&mut buffer2);
            let amount_ic: Nat = ROUTER.with(|router| {
                let r = router.borrow();
                let pool = r.get_pool(pool_id.clone()).unwrap();
                let native_deciaml: u8 = 
                    pool.get_token_by_chain_id(src_chain)
                        .map_or(
                            Err(format!("no according wrapper token for {} chain {} pool", src_chain, src_pool_id.clone())),
                            |token| Ok(token.token_local_decimals())
                        ).unwrap();
                pool.amount_evm_to_amount_ic(Nat::from(BigUint::from_bytes_le(&buffer2)), native_deciaml, wrapper_decimal)
            });

            let recipient_str = String::from_utf8(recipient.clone()).unwrap();
            let properly_trimmed_string = recipient_str.trim_matches(|c: char| c.is_whitespace() || c=='\0');
                            
            // the length of slice only be 0, 4, 29
            let recipient_addr: Principal = Principal::from_slice(properly_trimmed_string.as_bytes());

            // DIP20
            let transfer_res: CallResult<(TxReceipt, )> = ic_cdk::call(
                wrapper_token_addr,
                "mint",
                (recipient_addr, amount_ic),
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
                amount_evm.to_little_endian(&mut buffer3);
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
            amount_evm.to_little_endian(&mut buffer);
            handle_burn(dst_chain_id, dst_bridge_addr, recipient, buffer.to_vec()).await // how to handle failed transfer?
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
#[candid_method(update, rename = "burn_wrapper_token")]
async fn burn_wrapper_token(wrapper_token_addr: Principal, chain_id: u32, to: Vec<u8>, amount: Nat) -> Result<bool> {
    let caller = ic_cdk::caller();
    // DIP20
    let transfer_res: CallResult<(TxReceipt, )> = ic_cdk::call(
        wrapper_token_addr,
        "burn",
        (caller, amount.clone(), ),
    ).await;
    match transfer_res {
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

    let amount: Vec<u8> = BigUint::from(amount).to_bytes_le();
    let bridge_addr: Vec<u8> = get_bridge_addr(chain_id).unwrap();
    handle_burn(chain_id, bridge_addr, to, amount).await
}

// call proxy.get_nonce() to get nonce
async fn get_nonce(chain_id: u32, addr: String) -> Result<U256> {
    let proxy_canister: Principal = Principal::from_text(PROXY).unwrap();
    let call_res: CallResult<(Result<u64>, )> = ic_cdk::call(
        proxy_canister,
        "get_tx_count",
        (chain_id, addr,),
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
    let call_res: CallResult<(Result<u64>, )> = ic_cdk::call(
        proxy_canister,
        "get_gas_price",
        (chain_id,),
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

// call bridge.handleSwap
async fn handle_burn(chain_id: u32, bridge_addr: Vec<u8>, to: Vec<u8>, value: Vec<u8>) -> Result<bool> {
    // ecdsa key info
    let derivation_path = vec![ic_cdk::id().as_slice().to_vec()];
    let key_info = KeyInfo{ derivation_path: derivation_path, key_name: KEY_NAME.to_string() };

    let w3 = match ICHttp::new(URL, None, None) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { return Err(e.to_string()) },
    };
    let contract_address = Address::from_slice(&bridge_addr);
    let contract = Contract::from_json(
        w3.eth(),
        contract_address,
        BRIDGE_ABI
    ).map_err(|e| format!("init contract failed: {}", e))?;

    let c_addr = BRIDGE_ADDR.with(|addr| addr.borrow().clone());
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
    let to_addr = Address::from_slice(&to);
    let value = U256::from_little_endian(&value);
    let pool_id: u32 = 0;
    let signed = contract
        .sign("handleSwap", (pool_id, value, to_addr,), options, key_info, chain_id as u64)
        .await
        .map_err(|e| format!("sign handleSwap failed: {}", e))?;

    let raw_tx: Vec<u8> = signed.raw_transaction.0; 
    let proxy_canister: Principal = Principal::from_text(PROXY).unwrap();
    let call_res: CallResult<(TxReceipt, )> = ic_cdk::call(
        proxy_canister,
        "send_raw_tx",
        (chain_id, raw_tx),
    ).await;
    match call_res {
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

#[update(name = "add_bridge_addr")]
#[candid_method(update, rename = "add_bridge_addr")]
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
#[candid_method(update, rename = "remove_bridge_addr")]
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
#[candid_method(query, rename = "get_bridge_addr")]
fn get_bridge_addr(chain_id: u32) -> Result<Vec<u8>> {
    ROUTER.with(|router| {
        let r = router.borrow();
        r.get_bridge_addr(chain_id)
            .map_err(|_| format!("not bridge address in {} chain", chain_id))
    })
}

#[query(name = "is_bridge_addr_exist")]
#[candid_method(query, rename = "is_bridge_addr_exist")]
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

// #[cfg(not(any(target_arch = "wasm32", test)))]
fn main() {
    candid::export_service!();
    std::print!("{}", __export_service());
}

// #[cfg(any(target_arch = "wasm32", test))]
// fn main() {}


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

    fn add_new_pool(src_chain_id: u32, src_pool_id: Nat, symbol: String) -> bool {
        create_pool(src_chain_id, src_pool_id, symbol).is_ok()
    }

    fn add_wrapper_token(pool_id: Nat, wrapper_token_addr: String) -> bool {
        add_wrapper_token_addr(pool_id, wrapper_token_addr).unwrap_or(false)
    }

    #[test]
    #[ignore]
    fn should_create_pool() {
        let src_chain_id: u32 = 1; //ethereum
        let src_pool_id: Nat = 0.into(); // fake usdt pool id
        let symbol: String = "USDT".to_string();
        let res: bool = add_new_pool(src_chain_id, src_pool_id.clone(), symbol.clone());
        assert!(res);
        let pool_id = get_pool_id(src_chain_id, src_pool_id).unwrap();
        assert_eq!(pool_id, Nat::from(0));
        let pool_id = get_pool_id_by_symbol(symbol).unwrap();
        assert_eq!(pool_id, Nat::from(0));
    }

    #[test]
    #[ignore]
    fn should_add_wrapper_token_addr() {
        let src_chain_id: u32 = 1; //ethereum
        let src_pool_id: Nat = 0.into(); // fake usdt pool id
        assert!(add_new_pool(src_chain_id, src_pool_id.clone(), "USDT".to_string()));
        let wrapper_token_addr: &str = "aaaaa-aa"; //wrapper usdt canister address
        let pool_id = get_pool_id(src_chain_id, src_pool_id).unwrap();
        assert!(add_wrapper_token(pool_id, wrapper_token_addr.to_string()));
    }

    #[async_std::test]
    async fn should_process_swap_message() {
        let src_chain_id: u32 = 1; //ethereum
        let src_pool_id: Nat = 0.into(); // fake usdt pool id
        assert!(add_new_pool(src_chain_id, src_pool_id.clone(), "USDT".to_string()));
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

    #[async_std::test]
    async fn should_process_create_pool_message() {
        let src_chain_id: u32 = 5; //goerli ethereum
        let name: String = String::from("Fake USDT");
        let symbol: String = String::from("USDT");

        let token = vec![
            4u8.into_token(), // create_pool
            Token::Uint(Uint::from(0)), // src pool id = 0 (fake usdt)
            10u8.into_token(), // shared_decimals 10
            18u8.into_token(), // local_decimals 18 (real decimal of token on evm)
            Token::String(name), // token name
            Token::String(symbol.clone()), // token symbol
        ];
        let payload: Bytes = encode(&token);
        let sender: Vec<u8> = hex!("0000000000000000000000000000000000000000").into();

        let res: bool = process_message(src_chain_id, sender, 1, payload).await.unwrap();
        assert!(res);
        let pool_id: Nat = get_pool_id_by_symbol(symbol).unwrap();
        assert_eq!(pool_id, Nat::from(0));
    }


}