import { HardhatUserConfig } from "hardhat/config";
import "@nomicfoundation/hardhat-toolbox";

import * as dotenv from "dotenv";
dotenv.config();

const etherscanKey = process.env.ETHERSCAN_API_KEY;
const infuraKey = process.env.INFURA_API_KEY;
const alchemyKey = process.env.ALCHEMY_KEY;
const testPrivKey = process.env.TEST_PRIV_KEY;

module.exports = {
  solidity: {
    version: "0.8.9",
    settings: {
      optimizer: {
        enabled: true,
        runs: 999999,
      },
      metadata: {
        bytecodeHash: "none",
      },
    },
  },

  gasReporter: {
    currency: "USD",
  },

  networks: {
    localhost: {
      url: "http://localhost:8545",
    },
    kovan: {
      url: `https://kovan.infura.io/v3/${infuraKey}`,
      accounts: [testPrivKey]
    },
    rinkeby: {
      url: `https://rinkeby.infura.io/v3/${infuraKey}`,
      accounts: [testPrivKey]
    },
    goerli: {
      url: `https://goerli.infura.io/v3/${infuraKey}`,
      // url: `https://eth-goerli.g.alchemy.com/v2/${alchemyKey}`,
      accounts: [testPrivKey]
    },
	mumbai: { // polygon mumbai testnet
      url: `https://polygon-mumbai.g.alchemy.com/v2/${alchemyKey}`,
      accounts: [testPrivKey]
    },
    mainnet: {
      url: `https://mainnet.infura.io/v3/${infuraKey}`,
    },
  },

  typechain: {
    outDir: "./src",
    target: "ethers-v5",
    alwaysGenerateOverloads: false, // should overloads with full signatures like deposit(uint256) be generated always, even if there are no overloads?
  },

  etherscan: {
    apiKey: etherscanKey,
  },
};
