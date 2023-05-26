// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
//internal
import {TypeCasts} from "../utils/Utils.sol";
import {IOmnicReciver} from "../interfaces/IOmnicReciver.sol";
import {Omnic} from "../Omnic.sol";

contract DemoApp is Ownable {

    address public omnicAddr;

    // Message type
    uint8 public constant MESSAGE_TYPE_SYN = 0;
    uint8 public constant MESSAGE_TYPE_ACK = 1;
    uint8 public constant MESSAGE_TYPE_FAIL_ACK = 2;

    // ============ Events  ============
    event ReceivedMessage(
        uint32 indexed srcChainId,
        bytes32 srcSender,
        uint64 nonce,
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
        uint8 _msgType, //message type: {SYN, ACK, FAIL_ACK}
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bytes memory _payload
    ) public payable {
        // send message to dst chain, call omnic contract
        Omnic(omnicAddr).sendMessage{value: msg.value}(
            _msgType,
            _dstChainId,
            _recipientAddress,
            _payload,
            payable(msg.sender),
            address(0x0)
        );
    }

    // only omnic canister can call this func
    function handleMessage(
        uint8 msgType,
        uint32 srcChainId,
        bytes32 srcSender,
        uint64 nonce,
        bytes memory payload
    )
        public
        onlyOmnicContract
        returns (bool success)
    {
        if(msgType == MESSAGE_TYPE_SYN)
        {
            //
        } else if(msgType == MESSAGE_TYPE_ACK){
            //
        } else if(msgType == MESSAGE_TYPE_FAIL_ACK){
            //
        } else {
            //
        }
        // emit event when received message
        emit ReceivedMessage(srcChainId, srcSender, nonce, payload);
        return true;
    }
}