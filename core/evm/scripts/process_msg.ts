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
  // console.log("setting omnic canister addr...");
  // let tx = await omnic.setOmnicCanisterAddr("0x25816551E0E2e6FC256A0E7BCfFDFD1CA3CD390D");
  // console.log("tx:", tx);

  // call processMessage
  let msg = "0x000000000000000000000000000000000000000000000000000000000000000500000000000000000000000025816551e0e2e6fc256a0e7bcffdfd1ca3cd390d000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000138810000000000000000000000007e58df2620adda3ba6ff6aca989343d11807450e00000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000002068656c6c6f206f6d6e69632064656d6f20617070206f6e20706f6c79676f6e21";
  let data = ethers.utils.arrayify(msg);
  console.log('data:', data);
  let tx1 = await omnic.processMessage(data);
  console.log('tx1:', tx1);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
