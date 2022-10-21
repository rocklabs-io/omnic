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
  * fetch message merkle roots from external chains, for later message verification use
  * verify messages from offchain relayer, forward crosschain messages:
    * if message destination is IC, proxy canister notify the recipient canister on IC
    * if message destination is another chain, proxy canister create and sign a tx to the Omnic gateway contract on the destination chain, gateway contract will then notify the recipient contract
* Omnic offchain relayer:
  * fetch crosschain messages from supported chains; fetch known merkle roots from Omnic proxy canister
  * generate a merkle proof for each message then send the proof along with the message to the proxy canister
  * Why offchain: Proof generation can be computational intense & fetch messages via http calls in canister is expensive. If cost is low, relayer can also be a canister onchain.
* Applications
  * Application canisters living on IC
  * Application contracts living on external EVM chains



## 2. Code 

1. core: Omnic message passing protocol core implementation
2. omnic-bridge: A demo bridge app based on the Omnic messaging protocol
3. omnic-relayer: Omnic offchain relayer implementation
