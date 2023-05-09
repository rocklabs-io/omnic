// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

// Omnic Crosschain APP should implement the following interface

interface IOmnicReciver {
    /** Message type
     * SYN = 0: message send
     * ACK = 1: message response
     * FAIL_ACK = 2: message failure
     */

    function handleMessage(
        uint8 _msgType,
        uint32 _srcChainId,
        bytes32 _srcSenderAddress,
        uint64 _nonce,
        bytes calldata _payload
    ) external;
}
