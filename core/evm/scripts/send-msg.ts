import { ethers } from "hardhat";
import fs from "fs";
import { getContractAddr, getChainId } from "./helpers";
const hre = require("hardhat");

const send_msg = async function(chain: string, dst_chain: string, recipient: string, data: string) {
  const Omnic = await ethers.getContractFactory("Omnic");
  const beaconProxyAddr = getContractAddr(chain, "UpgradeBeaconProxy");
  const omnic = await Omnic.attach(beaconProxyAddr);

  console.log(`sending message from ${chain} to ${dst_chain}, recipient: ${recipient}, data: ${data}`);
  let tx = await omnic.sendMessage(
    getChainId(dst_chain), recipient, data
  );
  console.log("txhash:", tx.hash);
  console.log("new merkle root:", await omnic.getLatestRoot());
}

async function main() {
  const chain = hre.network.name;
  let dst_chain = "mumbai";
  let recipient_addr = "0x0e8F24712bc468170D1B24b64fA0A8a94871553B";
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
