
use ic_web3::ethabi::{decode, ParamType};

// use ic_web3::{
//     ethabi::ethereum_types::{U64, U256},
//     types::{Address, H256},
// };
use std::convert::TryInto;

type Result<T> = std::result::Result<T, String>;

pub fn get_operation_type(payload: &[u8]) -> Result<u8> {
    let t = vec![ParamType::Uint(8)];
    let d = decode(&t, &payload).map_err(|e| format!("payload decode error: {}", e))?;
    d[0]
        .clone()
        .into_uint()
        .ok_or("can not convert src_chain to U256")?
        .try_into()
        .map_err(|_| format!("convert U256 to u8 failed"))
}

// return (src_chain_id, src_pool_id, amount)
pub fn decode_operation_liquidity(payload: &[u8]) -> Result<(u32, u32, u128)> {
    /*
    uint8(OperationTypes.AddLiquidity), u8
    _srcChainId, u16
    _srcPoolId, u256
    _amount, u256
    */
    let types = vec![
        ParamType::Uint(8),
        ParamType::Uint(16),
        ParamType::Uint(256),
        ParamType::Uint(256),
    ];
    let d = decode(&types, payload).map_err(|e| format!("payload decode error: {} ", e))?;
    let src_chain_id: u32 = d[1]
        .clone()
        .into_uint()
        .ok_or("cannot convert src_chain to uint".to_string())?
        .as_u32();
    let src_pool_id: u32 = d[2]
        .clone()
        .into_uint()
        .ok_or("cannot convert src_pool to U256".to_string())?
        .as_u32();
    let amount: u128 = d[3]
        .clone()
        .into_uint()
        .ok_or("can not convert amount to U256".to_string())?
        .as_u128();
    Ok((src_chain_id, src_pool_id, amount))
}

// return (src_chain_id, src_pool_id, dst_chain_id, dst_pool_id, amount_ld, to)
pub fn decode_operation_swap(payload: &[u8]) -> Result<(u32, u32, u32, u32, u128, Vec<u8>)> {
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
    let d = decode(&types, payload).map_err(|e| format!("payload decode error: {}", e))?;
    let src_chain_id: u32 = d[1]
        .clone()
        .into_uint()
        .ok_or("cannot convert src_chain to U256".to_string())?
        .as_u32();
    let src_pool_id: u32 = d[2]
        .clone()
        .into_uint()
        .ok_or("can not convert src_pool_id to U256".to_string())?
        .as_u32();
    let dst_chain_id: u32 = d[3]
        .clone()
        .into_uint()
        .ok_or("can not convert dst_chain to U256".to_string())?
        .as_u32();
    let dst_pool_id: u32 = d[4]
        .clone()
        .into_uint()
        .ok_or("can not convert dst_pool_id to U256".to_string())?
        .as_u32();
    let amount: u128 = d[5]
        .clone()
        .into_uint()
        .ok_or("can not convert amount to U256".to_string())?
        .as_u128();
    let recipient: Vec<u8> = d[6]
        .clone()
        .into_fixed_bytes()
        .ok_or("can not convert recipient to bytes")?;
    Ok((src_chain_id, src_pool_id, dst_chain_id, dst_pool_id, amount, recipient))
}

// return (pool_id, shared_decimals, local_decimals, name, symbol)
pub fn decode_operation_create_pool(payload: &[u8]) -> Result<(u32, String, String, u8, u8, String, String)> {
    /*
        uint8(OperationTypes.CreatePool), u8
        _poolId, u256
        _poolAddr, address
        _tokenAddr, address
        _sharedDecimals, u8
        _localDecimals, u8
        _name, string
        _symbol, string
    */
    let types = vec![
        ParamType::Uint(8),
        ParamType::Uint(256),
        ParamType::Address,
        ParamType::Address,
        ParamType::Uint(8), // shared_decimals
        ParamType::Uint(8), // local_decimals
        ParamType::String,
        ParamType::String, 
    ];
    let d = decode(&types, payload).map_err(|e| format!("payload decode error: {}", e))?;

    let src_pool_id: u32 = d[1]
        .clone()
        .into_uint()
        .ok_or("cannot convert src_pool_id to U256".to_string())?
        .as_u32();
    let pool_addr: String = d[2]
        .clone()
        .into_address()
        .ok_or("cannot convert pool_address".to_string())?
        .to_string();
    let token_addr: String = d[3]
        .clone()
        .into_address()
        .ok_or("cannot convert token_address".to_string())?
        .to_string();
    let shared_decimal: u8 = d[4]
        .clone()
        .into_uint()
        .ok_or("cannot convert shared_decimals to U256".to_string())?
        .try_into().map_err(|_| format!("convert U256 to u8 failed"))?;
    
    let local_decimal: u8 = d[5]
        .clone()
        .into_uint()
        .ok_or("can not convert local_decimals U256".to_string())?
        .try_into().map_err(|_| format!("convert U256 to u8 failed"))?;
    let name: String = d[6]
        .clone()
        .into_string()
        .ok_or("can not convert name to String".to_string())?;
    let symbol: String = d[7]
        .clone()
        .into_string()
        .ok_or("can not convert symbol to String".to_string())?;
    Ok((src_pool_id, pool_addr, token_addr, shared_decimal, local_decimal, name, symbol))
}