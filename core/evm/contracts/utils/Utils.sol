// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.9;

library TypeCasts {

    // alignment preserving cast
    function addressToBytes32(address _addr) internal pure returns (bytes32) {
        return bytes32(uint256(uint160(_addr)));
    }

    // alignment preserving cast
    function bytes32ToAddress(bytes32 _buf) internal pure returns (address) {
        return address(uint160(uint256(_buf)));
    }
}

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
        uint8 _msg_type,
        uint32 _srcChainId,
        bytes32 _srcSenderAddress,
        uint64 _nonce,
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bytes memory _payload
    ) internal pure returns (bytes memory) {
        return
            abi.encode(
                _msg_type,
                _srcChainId,
                _srcSenderAddress,
                _nonce,
                _dstChainId,
                _recipientAddress,
                _payload
            );
    }
}