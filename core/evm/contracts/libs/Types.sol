// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.9;

/**
 * @title Types Library
 **/
library Types {

    /**
     * @notice Returns formatted (packed) message with provided fields
     * @param _srcChainId source chain id
     * @param _srcSenderAddress Address of sender, address length/format may vary by chains
     * @param _nonce Destination-specific nonce
     * @param _dstChainId  destination chain id
     * @param _recipientAddress Address of recipient on destination chain
     * @param _payload Raw bytes of message body
     **/
    struct MessageFormat {
        uint32 _srcChainId;
        bytes32 _srcSenderAddress;
        uint32 _nonce;
        uint32 _dstChainId;
        bytes32 _recipientAddress;
        bytes payload;
    }

    // alignment preserving cast
    function bytes32ToAddress(bytes32 _buf) internal pure returns (address) {
        return address(uint160(uint256(_buf)));
    }
}