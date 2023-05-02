import { ethers } from "hardhat";
import fs from "fs";
import { getContractAddr, getChainId } from "./helpers";
const hre = require("hardhat");

const send_msg = async function(chain: string, dst_chain: string, recipient: string, data: string) {
  const demoAddr = getContractAddr(chain, "Demo");
  const demo = await ethers.getContractAt("DemoApp", demoAddr);

  console.log(`sending message from ${chain} to ${dst_chain}, recipient: ${recipient}, data: ${data}`);
  let tx = await demo.sendMessage(getChainId(dst_chain), recipient, data, {value: ethers.utils.parseEther("0.001")});
  console.log("txhash:", tx.hash);
}

async function main() {
  const chain = hre.network.name;
  let dst_chain = "goerli";
  let recipient_addr = "0xcD5330aCf97E53489E3093Da52844e4D57b6Eae8";
  let recipient = ethers.utils.hexZeroPad(recipient_addr, 32);
  let data = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("hello omnic demo app on polygon!"));
  await send_msg(chain, dst_chain, recipient, data);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
