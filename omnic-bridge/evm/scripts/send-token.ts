import { ethers } from "hardhat";
import { getContractAddr } from "./helpers";
const hre = require("hardhat");

export const sendToken = async function (
  chain: string, 
  tokenSymbol: string,
  recipient: string,
  amount: number
  ) {
  const token = await ethers.getContractAt("ERC20", getContractAddr(chain, tokenSymbol));

  let tx = await token.transfer(recipient, amount);
  console.log("txhash:", tx.hash);
}

const main = async function () {
  let chain = hre.network.name;
  let recipient = "";
  let amount = 1000000000; // 1000 usdt
  await sendToken(chain, "USDT", recipient, amount);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
