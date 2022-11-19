// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.9;
pragma abicoder v2;

//external
import "@openzeppelin/contracts/utils/math/SafeMath.sol";

// internal
import "../utils/Buffer.sol";

/**
 * @title Message Library
 * @notice Library for formatted messages used by Omnic Node
 **/
library Message {
    using Buffer for Buffer.buffer;
    using SafeMath for uint256;

    // when decode data to Packet type
    struct Packet {
        uint32 srcChainId;
        bytes32 srcAddress;
        uint32 nonce;
        uint32 dstChainId;
        bytes32 recipientAddress;
        bytes messageBody;
    }

    // Number of bytes in formatted message before `body` field
    uint256 internal constant PREFIX_LENGTH = 76;

    /**
     * @notice Returns formatted (packed) message with provided fields
     * @param _srcChainId source chain id
     * @param _srcSenderAddress Address of sender, address length/format may vary by chains
     * @param _nonce Destination-specific nonce
     * @param _dstChainId  destination chain id
     * @param _recipientAddress Address of recipient on destination chain
     * @param _message Raw bytes of message body
     * @return Formatted message
     **/
    function formatMessage(
        uint32 _srcChainId,
        bytes32 _srcSenderAddress,
        uint64 _nonce,
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bytes memory _message
    ) internal pure returns (bytes memory) {
        return
            abi.encodePacked(
                _srcChainId,
                _srcSenderAddress,
                _nonce,
                _dstChainId,
                _recipientAddress,
                _message
            );
    }

    /**
     * @notice Returns leaf of formatted message with provided fields.
     * @param _srcChainId source chain id
     * @param _srcSenderAddress Address of sender, address length/format may vary by chains
     * @param _nonce Destination-specific nonce
     * @param _dstChainId  destination chain id
     * @param _recipientAddress Address of recipient on destination chain
     * @param _message Raw bytes of message body
     * @return Leaf (hash) of formatted message
     **/
    function messageHash(
        uint32 _srcChainId,
        bytes32 _srcSenderAddress,
        uint64 _nonce,
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bytes memory _message
    ) internal pure returns (bytes32) {
        return
            keccak256(
                formatMessage(
                    _srcChainId,
                    _srcSenderAddress,
                    _nonce,
                    _dstChainId,
                    _recipientAddress,
                    _message
                )
            );
    }

    function unpacketMessage(bytes memory data)
        internal
        pure
        returns (Packet memory)
    {
        /**
            Decode data from log to get detail information, e.g. source chain, destination address, message,
            the bytes length of the data：
                0-31   -> total bytes size
                32-35  -> source chain identifier
                36-67  -> source sender address
                68-71  -> nonce
                72-75  -> destination chain identifier
                76-107 -> destination address
                108--  -> message
         */

        uint256 totalSize;
        uint32 srcChainId;
        bytes32 srcSenderAddress;
        uint32 nonce;
        uint32 dstChainId;
        bytes32 recipientAddress;

        assembly {
            totalSize := mload(data)
            srcChainId := mload(add(data, 4))
            srcSenderAddress := mload(add(data, 36))
            nonce := mload(add(data, 40))
            dstChainId := mload(add(data, 44))
            recipientAddress := mload(add(data, 76))
        }

        uint messageSize = totalSize.sub(PREFIX_LENGTH);
        Buffer.buffer memory msgBuffer;
        msgBuffer.init(messageSize);
        msgBuffer.writeRawBytes(0, data, PREFIX_LENGTH.add(32), messageSize); // 76 + 32 = 108
        return
            Packet(
                srcChainId,
                srcSenderAddress,
                nonce,
                dstChainId,
                recipientAddress,
                msgBuffer.buf
            );
    }
}
