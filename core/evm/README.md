## Omnic EVM side contracts


### Build & deploy
install deps:
```
yarn install
```

compile:
```
npx hardhat compile
```

put your infura key & test private key to `.env`:
```
TEST_PRIV_KEY=0x...
TEST_ADDR=0x..
INFURA_API_KEY=...
ETHERSCAN_API_KEY=...
```

## Deploy
### deployment process:
  ```shell
  npx hardhat run scripts/deploy-feemanager.ts --network mumbai
  npx hardhat run scripts/deploy-impl.ts --network mumbai
  npx hardhat run scripts/deploy-proxy.ts --network mumbai
  ```

### upgrade process:
> if you want to upgrade omnic contract, please use this script to upgrade
1. upgrade-impl.ts


## Example
  ```shell
  // 1. deplot demo contract
  npx hardhat run scripts/deploy-demo.ts --network mumbai
  // 2. send message from mumbai to goerli
  npx hardhat run scripts/send-msg.ts --network mumbai
  ```