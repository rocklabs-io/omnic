import { ethers } from "hardhat";
import fs from "fs";
const hre = require("hardhat");
// need to solve Module not found error for this package
// import { Principal } from "@dfinity/principal";

async function main() {
  const chain = hre.network.name;
  let config = JSON.parse(fs.readFileSync('./config.json', 'utf-8'));

  const omnic_contract_addr = config.omnic_evm_contracts[chain];
  // const omnic_canister = Principal.fromText("rdmx6-jaaaa-aaaaa-aaadq-cai");

  const omnic = await ethers.getContractAt("Omnic", omnic_contract_addr);
  console.log("omnic address:", omnic.address);

  console.log("calling omnic.sendMessage...");
  let dest_chain = 80001; // send to Polygon testnet
  let recipient = "";
  // let recepient = ethers.utils.hexZeroPad(omnic_canister.toHex(), 32); // send to omnic canister
  let recepient = ethers.utils.hexZeroPad(recipient, 32);
  console.log("recepient:", recepient);
  let data = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("hello omnic app on polygon!"));
  let tx = await omnic.sendMessage(dest_chain, recepient, false, data);
  console.log("tx:", tx);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
