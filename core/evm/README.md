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

put your infura key & test private key to .env:
```
TEST_PRIV_KEY=0x...
TEST_ADDR=0x..
INFURA_API_KEY=...
ETHERSCAN_API_KEY=...
```

deploy to goerli:
```
npx hardhat run --network goerli scripts/deploy.ts
```

deployment process:
1. deploy-impl.ts
2. deploy-proxy.ts

upgrade process:
1. upgrade-impl.ts