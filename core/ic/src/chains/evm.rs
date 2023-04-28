use ic_web3::Web3;
use ic_web3::transports::ICHttp;
use ic_web3::types::{U256, H256, Bytes, Address, BlockNumber, BlockId};
use ic_web3::ic::KeyInfo;
use ic_web3::{
    contract::{Contract, Options},
    futures::StreamExt,
    types::FilterBuilder,
};
use hex_literal::hex;

use std::str::FromStr;
use async_trait::async_trait;

use crate::consts::KEY_NAME;
use crate::types::MessageStable;
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
    // only one item in msgs if dispatching just one message
    async fn dispatch_messages(&self, caller: String, dst_chain: u32, msg_bytes: Vec<Vec<u8>>) -> Result<H256, OmnicError> {
        let caller_addr = Address::from_str(&caller)
            .map_err(|e| Other(format!("address decode failed: {:?}", e)))?;
        // ecdsa key info
        let derivation_path = vec![ic_cdk::id().as_slice().to_vec()];
        let key_info = KeyInfo{ derivation_path: derivation_path, key_name: KEY_NAME.to_string() };
        // add nonce to options
        let tx_count = self.w3.eth()
            .transaction_count(caller_addr, None)
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
            op.gas_price = Some(gas_price);
        });
        ic_cdk::println!("gas price: {:?}", gas_price);
        let txhash = self.contract
            .signed_call("processMessageBatch", (msg_bytes,), options, caller, key_info, dst_chain as u64)
            .await
            .map_err(|e| ClientError(format!("processMessage failed: {}", e)))?;

        ic_cdk::println!("txhash: {}", hex::encode(txhash));

        Ok(txhash)
    }

    async fn send_raw_tx(&self, raw_tx: Vec<u8>) -> Result<Vec<u8>, OmnicError> {
        let raw = Bytes::from(raw_tx);
        self.w3.eth().send_raw_transaction(raw)
            .await
            .map(|res| res.as_bytes().to_vec())
            .map_err(|err| ClientError(format!("send_raw_tx failed: {:?}", err)))
    }

    async fn get_block_number(&self) -> Result<u64, OmnicError> {
        self.w3.eth().block_number()
            .await
            .map(|v| v.as_u64())
            .map_err(|e| ClientError(format!("get block number error: {:?}", e)))
    }

    async fn get_tx_count(&self, addr: String) -> Result<u64, OmnicError> {
        let addr = Address::from_str(&addr).map_err(|e| ClientError(format!("address convert faild: {:?}", e)))?;
        self.w3.eth().transaction_count(addr, None)
            .await
            .map(|v| v.as_u64())
            .map_err(|e| ClientError(format!("get tx count error: {:?}", e)))
    }

    async fn get_gas_price(&self) -> Result<u64, OmnicError> {
        self.w3.eth().gas_price()
            .await
            .map(|v| v.as_u64())
            .map_err(|e| ClientError(format!("get tx count error: {:?}", e)))
    }

    async fn scan_chunk(&self, start: u64, end: u64) -> Result<Vec<MessageStable>, OmnicError> {

        // Filter for SendMessage event in omnic gateway contract
        let filter = FilterBuilder::default()
        .address(vec![self.contract.address()])
        .topics(
            Some(vec![hex!(
                "d282f389399565f3671145f5916e51652b60eee8e5c759293a2f5771b8ddfd2e"
            )
            .into()]),
            None,
            None,
            None,
        )
        .build();

        let filter = self.w3.eth_filter().create_logs_filter(filter).await?;

        // todo: decode events from filter

        Err(OmnicError::Other("uncomplete".to_string()))
    }
}