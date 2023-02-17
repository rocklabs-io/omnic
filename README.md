# Omnic Crosschain Messaging Protocol

Omnic is a crosschain messaging protocol, powered by the Internet Computer's threshold ECDSA and outbound http call features, we can build programmability into the message passing process.

Currently support EVM compatible chains.

## 1. Architecture

![](./pics/arch.jpg)

Core components:

* Gateway contracts living on EVM chains:
  *  receive crosschain message requests from application contracts on local chain, messages are organized into a merkle tree; 
  * receive crosschain messages from external chains and notify recipient contracts on the local chain.
* Omnic proxy canister living on the IC:
  * verify messages from offchain relayer(using the merkle roots fetched by gateway canisters), forward crosschain messages:
    * if message destination is IC, proxy canister notify the recipient canister on IC
    * if message destination is another chain, proxy canister create and sign a tx to the Omnic gateway contract on the destination chain, gateway contract will then notify the recipient contract
* Omnic gateway canisters living on the IC:
  * each chain has a dedicated gateway canister, controlled by the Omnic proxy canister
  * responsible for periodically fetching message merkle roots from external chains, for later message verification use

* Omnic offchain relayer:
  * fetch crosschain messages from supported chains; fetch known merkle roots from Omnic proxy canister
  * generate a merkle proof for each message then send the proof along with the message to the proxy canister
  * Why offchain: Proof generation can be computational intense & fetch messages via http calls in canister is expensive. If cost is low, relayer can also be a canister onchain.
* Applications
  * Application canisters living on IC
  * Application contracts living on external EVM chains



## 2. TODOs

* Fix todos in code
* Add more chains
* Add examples
* ...



## 3. Deployment

Omnic test version is live on IC and 2 evm testnets:

Omnic proxy canister on IC mainnet: y3lks-laaaa-aaaam-aat7q-cai

Omnic gateway contract on EVM chains:

* Goerli: 0xc7D718dC3C9248c91813A98dCbFEC6CF57619520
* Mumbai: 0x2F711bEbA7a30242f4ba24544eA3869815c41413



## 4. Code 

1. [core](./core): Omnic message passing protocol core implementation
2. [examples](./examples): Example apps built on Omnic messaging protocol
3. [omnic-bridge](https://github.com/rocklabs-io/omnic-bridge): A demo bridge app based on the Omnic messaging protocol
4. [omnic-relayer](https://github.com/rocklabs-io/omnic-relayer): Omnic offchain relayer implementation



**Note: This project is unaudited and still in very early experimental stage!**
