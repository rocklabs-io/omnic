// SPDX-License-Identifier: MIT

pragma solidity ^0.8.9;

import {IOmnicReciver} from "./IOmnicReciver.sol";

interface IOmnic {
    struct Message {
        uint8 t; // message type: {SYN, ACK, FAIL_ACK}
        uint32 srcChainId; // message origin chain
        bytes32 srcSenderAddress; // sender on origin chain
        uint64 nonce; // app current nonce for destination chain
        uint32 dstChainId; // destination chain
        bytes32 recipient; // message recipient on destination chain
        bytes payload; // message data in bytes
    }

    // define CacheMessage to store failed transaction
    struct CacheMessage {
        uint64 msgLength;
        address dstAddress;
        bytes32 msgHash;
    }

    function sendMessage(
        uint8 _msgType, // message type
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bytes memory _payload,
        address payable _refundAddress,
        address _erc20PaymentAddress
    ) external payable;

    function sendMessageFree(
        uint8 _msgType, // message type
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bytes memory _payload
    ) external;

    function processMessage(
        bytes memory _message
    ) external returns (bool success);

    function processMessageBatch(
        bytes[] memory _messages
    ) external returns (bool success);

    function retryProcessMessage(
        uint8 t,
        uint32 _srcChainId,
        bytes32 _srcSenderAddress,
        bytes calldata _message
    ) external;

    function forceResumeReceive(
        uint32 _srcChainId,
        bytes32 _srcSenderAddress
    ) external;
}
