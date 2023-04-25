// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

// Omnic Crosschain APP should implement the following interface

interface IOmnicReciver {
    /** Message type
     * SYN: message send
     * ACK: message response
     * FAIL_ACK: message catch
     */
    enum MessageType {
        SYN,
        ACK,
        FAIL_ACK
    }

    function handleMessage(
        MessageType t,
        uint32 _srcChainId,
        bytes32 _srcSenderAddress,
        uint64 _nonce,
        bytes calldata payload
    ) external;
}
