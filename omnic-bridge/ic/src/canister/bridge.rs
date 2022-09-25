use candid::Principal;
use ic_cdk::export::candid::{candid_method, Nat};
use ic_cdk_macros::{query, update};
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
use omnic_bridge::token::Token as BrideToken;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::convert::TryInto;

ic_cron::implement_cron!();

const OPERATION_ADD_LIQUIDITY: u8 = 1u8;
const OPERATION_REMOVE_LIQUIDITY: u8 = 2u8;
const OPERATION_SWAP: u8 = 3;

const OWNER: &'static str = "aaaaa-aa";

const URL: &str = "https://goerli.infura.io/v3/93ca33aa55d147f08666ac82d7cc69fd";
const KEY_NAME: &str = "dfx_test_key";
const TOKEN_ABI: &[u8] = include_bytes!("./bridge.json");

type Result<T> = std::result::Result<T, String>;


thread_local! {
    static ROUTER: RefCell<Router<Vec<u8>>> = RefCell::new(Router::new());
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
    } else {
        Err("unsupported!".to_string())
    }
}

#[update(name = "send_message")]
#[candid_method(update, rename = "sendMessage")]
fn send_message() -> Result<bool> {
    //TODO
    Ok(false)
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
    let txhash = contract
        .signed_call("transfer", (to_addr, value,), options, key_info, chain_id as u64)
        .await
        .map_err(|e| format!("token transfer failed: {}", e))?;

    ic_cdk::println!("txhash: {}", hex::encode(txhash));

    Ok(true)
}

#[update(name = "create_pool")]
#[candid_method(update, rename = "createPool")]
fn create_pool(src_chain: u32, src_pool_id: Nat) -> Result<bool> {
    let caller: Principal = ic_cdk::caller();
    let owner: Principal = Principal::from_text(OWNER).unwrap();
    assert_eq!(caller, owner);

    ROUTER.with(|router| {
        let mut r = router.borrow_mut();
        let pool_id: Nat = r.get_pools_length();
        let tokens: BTreeMap<u32, BrideToken<Vec<u8>>> = BTreeMap::new();
        let pool = Pool::new(pool_id.clone(), tokens);
        r.add_pool(pool)
            .map_err(|e| format!("create pool failed: {}", e))?;
        r.add_pool_id(src_chain, src_pool_id)
            .map_err(|e| format!("create pool failed: {}", e))
    })
}

#[update(name = "add_supported_token")]
#[candid_method(update, rename = "addSupportedToken")]
fn add_supported_token(
    src_chain: u32,
    src_pool_id: Nat,
    name: String,
    symbol: String,
    local_decimals: u8,
    shared_decimals: u8,
) -> Result<bool> {
    let caller: Principal = ic_cdk::caller();
    let owner: Principal = Principal::from_text(OWNER).unwrap();
    assert_eq!(caller, owner);

    ROUTER.with(|router| {
        let mut r = router.borrow_mut();
        let pool_id: Nat = r
            .get_pool_id(src_chain.clone(), src_pool_id.clone())
            .map_err(|e| format!("{}", e))?;
        let mut pool = r.get_pool(pool_id.clone()).map_err(|e| format!("{}", e))?;
        let balances: BTreeMap<Vec<u8>, Nat> = BTreeMap::new();
        let token = BrideToken::new(
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
