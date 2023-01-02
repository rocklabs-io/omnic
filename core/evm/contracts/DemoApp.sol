// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

//internal
import {QueueLib} from "./libs/Queue.sol";
import {QueueManager} from "./utils/QueueManager.sol";
import {MerkleLib} from "./libs/Merkle.sol";
import {Types} from "./libs/Types.sol";
import {TypeCasts} from "./utils/Utils.sol";
import {IOmnicReciver} from "./interfaces/IOmnicReciver.sol";

import {Omnic} from "./Omnic.sol";

contract DemoApp {

    address owner;
    address omnicAddr;

    // ============ Events  ============
    event ReceivedMessage(
        uint32 indexed srcChainId,
        bytes32 srcSender,
        uint32 nonce,
        bytes payload
    );

    modifier onlyOmnicContract() {
        require(msg.sender == omnicAddr, "!omnicContract");
        _;
    }

    modifier onlyOwner() {
        require(msg.sender == owner, "!owner");
        _;
    }

    constructor(address omnic) {
        owner = msg.sender;
        omnicAddr = omnic;
    }

    function setOmnicContractAddr(address _newAddr)
        public
        onlyOwner
    {
        omnicAddr = _newAddr;
    }

    function sendMessage(
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bytes memory _payload
    ) public payable {
        // send message to dst chain, call omnic contract
        Omnic(omnicAddr).sendMessage(
            _dstChainId,
            _recipientAddress,
            _payload,
            payable(msg.sender),
            address(this)
        );
    }

    // only omnic canister can call this func
    function handleMessage(
        uint32 srcChainId,
        bytes32 srcSender,
        uint32 nonce,
        bytes memory payload
    )
        public
        onlyOmnicContract
        returns (bool success)
    {
        // emit event when received message
        emit ReceivedMessage(srcChainId, srcSender, nonce, payload);
        return true;
    }
}
