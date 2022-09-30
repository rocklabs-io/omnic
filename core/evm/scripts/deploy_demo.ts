import { ethers } from "hardhat";
import fs from 'fs';
const hre = require("hardhat");

const main = async function () {
  let chain = hre.network.name;
  let config = JSON.parse(fs.readFileSync('./config.json', 'utf-8'));
  let omnic_addr = config.omnic_evm_contracts[chain];

  const DemoApp = await ethers.getContractFactory("DemoApp");
  const demo = await DemoApp.deploy(omnic_addr);

  await demo.deployed();
  console.log("DemoApp deployed to:", demo.address);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
