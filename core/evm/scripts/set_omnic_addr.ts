import { ethers } from "hardhat";
import fs from "fs";
const hre = require("hardhat");
// need to solve Module not found error for this package
// import { Principal } from "@dfinity/principal";

async function main() {
  const chain = hre.network.name;
  let config = JSON.parse(fs.readFileSync('./config.json', 'utf-8'));

  const omnic_contract_addr = config.omnic_evm_contracts[chain];

  const omnic = await ethers.getContractAt("Omnic", omnic_contract_addr);

  // set omnic canister addr to omnic contract
  console.log("setting omnic canister addr...");
  let tx = await omnic.setOmnicCanisterAddr(config.omnic_canister_addr);
  console.log("tx:", tx);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
