import { time, loadFixture } from "@nomicfoundation/hardhat-network-helpers";
import { anyValue } from "@nomicfoundation/hardhat-chai-matchers/withArgs";
import { expect } from "chai";
import { ethers } from "hardhat";

describe("Omnic", function () {
  // We define a fixture to reuse the same setup in every test.
  // We use loadFixture to run this setup once, snapshot that state,
  // and reset Hardhat Network to that snapshopt in every test.

  // describe("Deployment", function () {
  //   it("Should set the right unlockTime", async function () {
  //     const { lock, unlockTime } = await loadFixture(deployOneYearLockFixture);

  //     expect(await lock.unlockTime()).to.equal(unlockTime);
  //   });
  // });
	describe("SendMessage", function() {
		it("Should send msg", async function () {
			const Omnic = await ethers.getContractFactory("Omnic");
			const omnic = await Omnic.deploy();

			let recipient_addr = "0xcD5330aCf97E53489E3093Da52844e4D57b6Eae8";
			let recipient = ethers.utils.hexZeroPad(recipient_addr, 32);
			let data = ethers.utils.hexlify(ethers.utils.toUtf8Bytes("hello omnic demo app on polygon!"));
			const tx = await omnic.sendMessage(0, recipient, data);
			//const tx1 = await omnic.processMessage(recipient + data.slice(2))
		});
	});
});
