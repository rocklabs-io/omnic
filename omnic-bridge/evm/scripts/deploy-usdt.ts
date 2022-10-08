import { ethers } from "hardhat";
import { getContractAddr, updateConfig } from "./helpers";
const hre = require("hardhat");

export const deployToken = async function (chain: string, name: string, symbol: string, decimals: number, supply: number) {
  const Token = await ethers.getContractFactory("ERC20");

  const tokenAddr = getContractAddr(chain, symbol);
  let token;
  if(tokenAddr == null) {
    console.log("deploying test token...");
    token = await Token.deploy(name, symbol, decimals, supply);

    await token.deployed();
    console.log("chain: ", chain, "test token deployed to:", token.address);
    updateConfig(chain, symbol, token.address);
  } else {
    console.log("found deployed token:", tokenAddr);
    token = await ethers.getContractAt("Omnic", tokenAddr);
  }
  return token;
}

const main = async function () {
  let chain = hre.network.name;
  await deployToken(chain, "USDT test", "USDT", 6, 100_000_000_000_000);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
