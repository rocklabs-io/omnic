// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

// Omnic Crosschain APP should implement the following interface

interface IMessageRecipient {
    function handleMessage(
        uint32 _srcChainId,
        bytes32 _srcSenderAddress,
        uint32 _nonce,
        bytes calldata payload
    ) external;
}