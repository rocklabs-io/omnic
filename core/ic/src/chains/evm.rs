use ic_web3::Web3;
use ic_web3::transports::ICHttp;
use ic_web3::contract::{Contract, Options};
use ic_web3::types::{H256, Address, BlockNumber, BlockId};

use std::str::FromStr;
use async_trait::async_trait;

use crate::types::Message;
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
            .map_err(|e| Other("address decode failed!".into()))?;
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
    async fn dispatch_message(&self, caller: String, msg: &Message) -> Result<Option<H256>, OmnicError> {
        unimplemented!();
        // // add nonce to options
        // let tx_count = w3.eth()
        //     .transaction_count(canister_addr, None)
        //     .await
        //     .map_err(|e| format!("get tx count error: {}", e))?;
        // // get gas_price
        // let gas_price = w3.eth()
        //     .gas_price()
        //     .await
        //     .map_err(|e| format!("get gas_price error: {}", e))?;
        // // legacy transaction type is still ok
        // let options = Options::with(|op| { 
        //     op.nonce = Some(tx_count);
        //     op.gas_price = Some(gas_price);
        //     op.transaction_type = Some(U64::from(2)) //EIP1559_TX_ID
        // });
        // let to_addr = Address::from_str(&addr).unwrap();
        // let txhash = contract
        //     .signed_call("transfer", (to_addr, value,), options, key_info, CHAIN_ID)
        //     .await
        //     .map_err(|e| format!("token transfer failed: {}", e))?;

        // ic_cdk::println!("txhash: {}", hex::encode(txhash));

        // Ok(format!("{}", hex::encode(txhash)))
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
            Err(e) => Err(Other(format!("get root error: {:?}", e)))
        }
    }

    async fn get_block_number(&self) -> Result<u64, OmnicError> {
        self.w3.eth().block_number()
            .await
            .map(|v| v.as_u64())
            .map_err(|e| Other(format!("get block number error: {:?}", e)))
    }
}