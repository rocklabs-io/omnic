## Omnic Core Spec



### 1. Data Spec

#### 1.1 Domains

Different chains are refered to as domains, each with an id, type `u32`, used for specify the message destination:

| Domain   | ID   |
| -------- | ---- |
| IC       | 0    |
| Ethereum | 1    |
| Arbitrum | 2    |

More domains to be added.

#### 1.2 Message format

A crosschain message must contain the following data:

```
uint32 originDomain // message origin domain ID
uint32 nonce // current nonce for destination chain
uint32 destinationDomain // destination domain ID
bytes32 recipientAddress // message recipient
bytes messageBody // message data in bytes
```

If the destinationDomain is 0, then it's a message sent to canister `recipientAddress` on the IC.



### 2. EVM gateway contract

Ref: https://github.com/nomad-xyz/monorepo/tree/main/packages/contracts-core/contracts

EVM side gateway contract receives messages from application contracts, enqueue the messages:

```
enqueue_message(destinationDomain, receipientAddress, data) // called by application contracts, enqueue a crosschain message
process_message(message: Message) // called by Omnic proxy canister, process an message from another chain, should record nonce to prevent replay attack; this function will call destination contranct's handle_message function
```

Application contracts must implement `handle_message` to be able to receive crosschain message:

```
handle_message(message: Message) // handle the crosschain message on the application side
```



### 3. IC proxy canister

The IC side proxy canister helps relay crosschain messages, periodically fetch crosschain messages and process them.

```
// enqueue_tx(tx: Vec<u8>, destinationDomain: u32); // application canister enqueue a signed raw tx to be sent to the destination chain. Or we let app canisters send tx themselves?
process_message(message: Message); // process a crosschain message, if the destination is IC, call the destination canister's handle_message function; if dest is another chain, send the message to another chain by sending a transaction
```



in `heart_beat`:

```
fetch_messages(domainId: u32); // fetch new crosschain message from all supported chains in a loop
batch_send_tx(); // send crosschain messages
batch_query_tx_status(); // query tx status and mark them
```



### 4. Omnic bridge

Lock & mint style bridge, support EVM to IC, EVM to EVM.

#### 4.1 IC-ETH

Bridge contract: 

```
lock(token: Address, amount: u256, dest: u32, recepient: bytes32); // lock some token, construct a crosschain message, call gateway contract's enqueue_message
unlock(token: Address, amount: u256, recepient: Address); // called by the bridge canister, release some token on Ethereum
```

Bridge canister:

```
handle_message(); // receive crosschain message from omnic proxy canister, call mint function to mint tokens to users
mint(token: Principal, amount: u64, recepient: Principal); // mint some token on IC for user
burn(token: Principal, amount: u64, recepient: blob); // burn some token on IC, bridge back to Ethereum, send a transaction to Eth to release the tokens for user
```



#### 4.2 ETH-Arbitrum

