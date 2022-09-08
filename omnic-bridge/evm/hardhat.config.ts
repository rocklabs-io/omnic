import "@nomicfoundation/hardhat-toolbox";

module.exports = {
  solidity: "0.8.9",
  typechain: {
    outDir: "./src",
    target: "ethers-v5",
    alwaysGenerateOverloads: false,
  },
};
