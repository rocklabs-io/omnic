// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
//internal
import {TypeCasts} from "../utils/Utils.sol";
import {IOmnicReciver} from "../interfaces/IOmnicReciver.sol";
import {Omnic} from "../Omnic.sol";

contract DemoApp is Ownable {

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

    constructor(address omnic) {
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
            address(0x0)
        );
    }

    // only omnic canister can call this func
    function handleMessage(
        IOmnicReciver.MessageType t,
        uint32 srcChainId,
        bytes32 srcSender,
        uint32 nonce,
        bytes memory payload
    )
        public
        onlyOmnicContract
        returns (bool success)
    {
        if(t == IOmnicReciver.MessageType.SYN)
        {
            //
        } else if(t == IOmnicReciver.MessageType.ACK){
            //
        } else if(t == IOmnicReciver.MessageType.FAIL_ACK){
            //
        } else {
            //
        }
        // emit event when received message
        emit ReceivedMessage(srcChainId, srcSender, nonce, payload);
        return true;
    }
}