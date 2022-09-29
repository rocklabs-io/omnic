## Omnic Crosschain Messaging Protocol

testing process:
1. deploy Omnic.sol & DemoApp.sol on goerli & polygon-mumbai
2. deploy omnic canister and demo canister
3. set omnic canister's address to Omnic.sol on both chains
4. setup & start omnic-relayer
5. send a message from DemoApp on goerli to ic
6. send a message from DemoApp on goerli to polygon-mumbai
7. check if the message is received on IC demo canister
8. check if message is relayed to polyon-mumbai DemoApp
