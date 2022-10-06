import { ethers } from "hardhat";
import { getContractAddr } from "./helpers";
const hre = require("hardhat");

export const createPool = async function (chain: string, tokenSymbol: string) {
  const token = await ethers.getContractAt("ERC20", getContractAddr(chain, tokenSymbol));
  const router = await ethers.getContractAt("BridgeRouter", getContractAddr(chain, "BridgeRouter"));

  /*
    address _token,
    uint8 _sharedDecimals,
    uint8 _localDecimals,
    string memory _name,
    string memory _symbol
  */
  let tx = await router.createPool(
      token.address, 
      await token.decimals(),
      await token.decimals(),
      await token.name(),
      await token.symbol()
      );
  console.log("createPool tx:", tx.hash);
}

const main = async function () {
  let chain = hre.network.name;
  await createPool(chain, "USDT");
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
