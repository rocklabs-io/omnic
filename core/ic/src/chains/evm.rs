use ic_web3::Web3;
use ic_web3::transports::ICHttp;
use ic_web3::contract::{Contract, Options};
use ic_web3::types::{U64, U256, H256, Address, BlockNumber, BlockId};
use ic_web3::ic::KeyInfo;

use std::str::FromStr;
use async_trait::async_trait;

use crate::consts::KEY_NAME;
use crate::error::OmnicError;
use crate::error::OmnicError::*;
use crate::traits::chain::HomeContract;

const OMNIC_ABI: &[u8] = include_bytes!("./omnic.abi");

pub struct EVMChainClient {
    w3: Web3<ICHttp>,
    contract: Contract<ICHttp>,
}

impl EVMChainClient {
    pub fn new(
        rpc_url: String,
        omnic_addr: String,
        max_resp_bytes: Option<u64>,
        cycles_per_call: Option<u64>,
    ) -> Result<EVMChainClient, OmnicError> {
        let http = ICHttp::new(&rpc_url, max_resp_bytes, cycles_per_call)?;
        let w3 = Web3::new(http);
        let contract_address = Address::from_str(&omnic_addr)
            .map_err(|e| Other(format!("address decode failed: {:?}", e)))?;
        let contract = Contract::from_json(
            w3.eth(),
            contract_address,
            OMNIC_ABI
        )?;

        Ok(EVMChainClient {
            w3: w3,
            contract: contract,
        })
    }
}

#[async_trait]
impl HomeContract for EVMChainClient {
    async fn dispatch_message(&self, caller: String, dst_chain: u32, msg_bytes: Vec<u8>) -> Result<H256, OmnicError> {
        let caller = Address::from_str(&caller)
            .map_err(|e| Other(format!("address decode failed: {:?}", e)))?;
        // ecdsa key info
        let derivation_path = vec![ic_cdk::id().as_slice().to_vec()];
        let key_info = KeyInfo{ derivation_path: derivation_path, key_name: KEY_NAME.to_string() };
        // add nonce to options
        let tx_count = self.w3.eth()
            .transaction_count(caller, None)
            .await
            .map_err(|e| ClientError(format!("get tx count error: {}", e)))?;
        // get gas_price
        let gas_price = self.w3.eth()
            .gas_price()
            .await
            .map_err(|e| ClientError(format!("get gas_price error: {}", e)))?;
        // legacy transaction type is still ok
        let options = Options::with(|op| { 
            op.gas = Some(U256::from(100000));
            op.nonce = Some(tx_count);
            // op.gas_price = Some(gas_price);
            op.gas_price = Some(U256::from(20));
            // op.transaction_type = Some(U64::from(2)) //EIP1559_TX_ID
        });
        ic_cdk::println!("gas price: {:?}", gas_price);
        let txhash = self.contract
            .signed_call("processMessage", (msg_bytes,), options, key_info, dst_chain as u64)
            .await
            .map_err(|e| ClientError(format!("processMessage failed: {}", e)))?;

        ic_cdk::println!("txhash: {}", hex::encode(txhash));

        Ok(txhash)
    }

    async fn get_latest_root(&self, height: Option<u64>) -> Result<H256, OmnicError> {
        // query root in block height
        let h = match height {
            Some(v) => BlockId::Number(BlockNumber::Number(v.into())),
            None => BlockId::Number(BlockNumber::Latest),
        };
        let root: Result<H256, ic_web3::contract::Error> = self.contract
            .query(
                "getLatestRoot", (), None, Options::default(), 
                h
            )
            .await;
        match root {
            Ok(r) => Ok(r),
            Err(e) => Err(ClientError(format!("get root error: {:?}", e)))
        }
    }

    async fn get_block_number(&self) -> Result<u64, OmnicError> {
        self.w3.eth().block_number()
            .await
            .map(|v| v.as_u64())
            .map_err(|e| ClientError(format!("get block number error: {:?}", e)))
    }
}