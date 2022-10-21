## Omnic Core Spec



### 1. Data Spec

#### 1.1 Domains

Different chains are refered to as domains, each with an id, type `u32`, used for specify the message destination:

| Domain          | ID    |
| --------------- | ----- |
| IC              | 0     |
| Ethereum        | 1     |
| Ethereum Goerli | 5     |
| Polygon Mumbai  | 80001 |

More domains to be added.

#### 1.2 Message format

Crosschain message format:

```
uint32 origin // message origin chain
bytes32 sender // sender on origin chain
uint32 nonce // current nonce for destination chain
uint32 destination // destination chain
bytes32 recipient // message recipient on destination chain
bytes payload // message data in bytes
```

If the destination is 0, then it's a message sent to canister `recipient` on the IC.

The sender & recipient are padded into `bytes32` format, principal IDs should be converted into bytes format first then left padded with zeros.

### 2. EVM gateway contract

EVM side gateway contract receives messages from application contracts, organize messages into a merkle tree, core interfaces:

```
sendMessage(uint32 destination, bytes32 receipient, bytes memory payload) // called by application contracts, enqueue a crosschain message
processMessage(bytes memory message) // called by Omnic proxy canister, process an incoming message from another chain, this function will call recipient contranct's handleMessage function
```

Application contracts must implement `handleMessage` to be able to receive crosschain message from Omnic gateway contract:

```
handleMessage(uint32 origin, bytes32 sender, uint32 nonce, bytes memory payload) // handle the crosschain message on the application side
```



### 3. IC proxy canister

The IC side proxy canister periodically fetch message merkle roots from supported chains, verify crosschain messages submitted by offchain relayer, and process crosschain messages:

* if the message destination is IC, notify the recipient canister
* if the message destination is another chain, construct and sign a tx to the Omnic gateway contract on the destination chain, tx is signed via threshold ECDSA, and sent by outbound http call

```
get_latest_root(chain_id: u32) -> Result<String, String> // get latest merkle root for given chain
process_message(message: Vec<u8>, proof: Vec<Vec<u8>>, leaf_index: u32) -> Result<bool, String> // called by the offchain relayer, verify & process a crosschain message
```

In order to receive crosschain message notification, application canisters must implement `handle_message` function:

```
handle_message(origin: u32, nonce: u32, sender: Vec<u8>, body: Vec<u8>) -> Result<bool, String>
```



