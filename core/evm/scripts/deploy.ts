import { ethers } from "hardhat";

async function main() {
  const Omnic = await ethers.getContractFactory("Omnic");
  const omnic = await Omnic.deploy();

  await omnic.deployed();

  console.log("omnic deployed to:", omnic.address);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
