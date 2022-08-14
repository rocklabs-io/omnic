// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

// Omnic Crosschain APP should implement the following interface

interface IMessageRecipient {
    function handleMessage(
        uint32 _origin,
        uint32 _nonce,
        bytes32 _sender,
        bytes memory _message
    ) external;
}