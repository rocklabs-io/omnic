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
     * @return Formatted message
     **/
    function formatMessage(
        uint32 _srcChainId,
        bytes32 _srcSenderAddress,
        uint32 _nonce,
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bool _wait_optimistic,
        bytes memory _payload
    ) internal pure returns (bytes memory) {
        return
            abi.encodePacked(
                _srcChainId,
                _srcSenderAddress,
                _nonce,
                _dstChainId,
                _recipientAddress,
                _wait_optimistic,
                _payload
            );
    }
}
