// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

// Omnic Crosschain APP should implement the following interface

interface IOmnicReciver {
    /**
     * @notice Omnic endpoint will invoke this function to deliver the message on the destination (e.g. user application)
     * @param _srcChainId        <! the source chain identifier, e.g. ethereum = 1,
     * @param _srcSenderAddress  <! the source contract (as bytes32) at the source.
     * @param _nonce             <! the message nonce.
     * @param _message           <! the message as bytes is encoded by user application to be sent.
     */
    function handleMessage(
        uint32 _srcChainId,
        bytes32 _srcSenderAddress,
        uint64 _nonce,
        bytes calldata _message
    ) external;
}
