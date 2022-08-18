import { ethers } from "hardhat";
// need to solve Module not found error for this package
// import { Principal } from "@dfinity/principal";

async function main() {
  const omnic_addr = "0x0fA355bEEA41d190CAE64F24a58F70ff2912D7df";
  // const omnic_canister = Principal.fromText("rdmx6-jaaaa-aaaaa-aaadq-cai");

  const omnic = await ethers.getContractAt("Omnic", omnic_addr);
  console.log("omnic address:", omnic.address);

  console.log("calling omnic.enqueueMessage...");
  let dest_chain = 0; // send to IC
  // let recepient = ethers.utils.hexZeroPad(omnic_canister.toHex(), 32); // send to omnic canister
  let recepient = ethers.utils.hexZeroPad(omnic_addr, 32);
  console.log("recepient:", recepient);
  let data = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("hello omnic!"));
  let tx = await omnic.enqueueMessage(dest_chain, recepient, data);
  console.log("tx:", tx);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
