import { ethers } from "hardhat";
import { getContractAddr, getNonce } from "./helpers";
const hre = require("hardhat");

export const createPool = async function (chain: string, tokenSymbol: string) {
  const token = await ethers.getContractAt("ERC20", getContractAddr(chain, tokenSymbol));
  const router = await ethers.getContractAt("Router", getContractAddr(chain, "Router"));

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
      await token.symbol(),
      {nonce: await getNonce()}
      );
  console.log("createPool tx:", tx.hash);
  let res = await tx.wait();
  console.log("res:", res);
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
