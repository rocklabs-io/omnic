import { ethers } from "hardhat";
import fs from 'fs';
const hre = require("hardhat");

const main = async function () {
  let chain = hre.network.name;
  const Omnic = await ethers.getContractFactory("Omnic");
  const omnic = await Omnic.deploy();

  await omnic.deployed();
  console.log("omnic deployed to:", omnic.address);

  // recording omnic contract address
  let config = JSON.parse(fs.readFileSync('./config.json', 'utf-8'));
  config.omnic_evm_contracts[chain] = omnic.address;
  fs.writeFileSync("config.json", JSON.stringify(config));

  // set omnic canister addr to omnic contract
  console.log("setting omnic canister addr...");
  console.log(config.omnic_canister_addr)
  let tx = await omnic.setOmnicCanisterAddr(config.omnic_canister_addr);
  console.log("tx:", tx);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
