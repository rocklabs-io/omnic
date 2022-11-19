// SPDX-License-Identifier: MIT

pragma solidity ^0.8.9;

/**
 * @dev This interface defines the fabricity of the Endpoint.
 */
interface IOmnicEndpoint {
    /**
     * @notice User applications invoke it to send a message to the specified address at a Omnic endpoint.
     * @param _dstChainId           <! the destination chain id, e.g. ethereum = 1,
     * @param _recipientAddress     <! the address on destination chain (using bytes32 to adapt other different address format).
     * @param _message              <! a custom bytes message to send to the destination contract
     * @param _refundAddress        <! if the source transaction is cheaper than the amount of value passed, refund the additional amount to this address
     * @param _ERC20PaymentAddress  <! using ERC20 (specified by omnic) token to pay for the transaction
     */
    function sendMessage(
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bytes calldata _message,
        address payable _refundAddress,
        address _ERC20PaymentAddress
    ) external payable;

    /**
     * @notice This function processes a message from other chain to the destination address on local chain.
     * @param _srcChainId         <! the source chain identifier, e.g. ethereum = 1,
     * @param _srcSenderAddress   <! the source contract (as bytes32) at the source.
     * @param _dstAddress         <! the address (e.g. user application) on local chain.
     * @param _nonce              <! the unbound message ordering nonce
     * @param _gasLimit           <! the gas limit for external contract execution
     * @param _message            <! verified payload to send to the local contract
     */
    function processMessage(
        uint32 _srcChainId,
        bytes32 _srcSenderAddress,
        address _dstAddress,
        uint64 _nonce,
        uint _gasLimit,
        bytes calldata _message
    ) external;

    /**
     * @notice the interface to retry failed message on this Endpoint destination
     * @param _srcChainId        <! the source chain identifier, e.g. ethereum = 1,
     * @param _srcSenderAddress  <! the source chain contract sender address
     * @param _message           <! the message to be retried
     */
    function retryProcessMessage(
        uint32 _srcChainId,
        bytes32 _srcSenderAddress,
        bytes calldata _message
    ) external;

    /**
     * @notice User application use this func to clear the cache message
     * @param _srcChainId        <! the source chain identifier, e.g. ethereum = 1,
     * @param _srcSenderAddress  <! the source chain contract sender address
     */
    function forceResumeReceive(uint32 _srcChainId, bytes32 _srcSenderAddress)
        external;

    /**
     * @notice gets a quote in source native gas, for the amount that send() requires to pay for message delivery
     * @param _message          <! the custom message to send over Omnic
     * @param _payInERC20       <! if false, user app pays the protocol fee in native token
     * @return nativeFee        <! fee with native token
     * @return erc20Fee         <! fee with erc20 token
     */
    function estimateFees(
        bytes calldata _message,
        bool _payInERC20
    ) external view returns (uint256 nativeFee, uint256 erc20Fee);

    /**
     * @notice get the inboundNonce of a receiver from a source chain which could be EVM or non-EVM chain
     * @param _srcChainId - the source chain identifier
     * @param _srcSenderAddress - the source chain contract address
     */
    function getInboundNonce(uint32 _srcChainId, bytes32 _srcSenderAddress)
        external
        view
        returns (uint64);

    /**
     * @notice get the inboundNonce of a receiver from a source chain which could be EVM or non-EVM chain
     * @param _dstChainId - the destination chain id, e.g. ethereum = 1,
     * @param _srcSenderAddress - the source chain contract address
     */

    function getOutboundNonce(uint32 _dstChainId, address _srcSenderAddress)
        external
        view
        returns (uint64);

    /** @notice get this Endpoint's immutable source identifier */
    function getChainId() external view returns (uint32);

    /** @notice get this OmnicNode's address */
    function getOmnicNode() external view returns (address);

    /**
     * @notice query if the non-reentrancy guard for sendMessage() is on
     * @return true if the guard is on. false otherwise
     */
    function isSendingMessage() external view returns (bool);

    /**
     * @notice query if the non-reentrancy guard for processMessage() is on
     * @return true if the guard is on. false otherwise
     */
    function isProcessingMessage() external view returns (bool);

    /**
     * @notice query if any STORED payload (message blocking) at the endpoint.
     * @param _srcChainId - the source chain identifier
     * @param _srcSenderAddress - the source chain contract address
     * @return true if message is in catch
     */

    function hasCacheMessage(uint32 _srcChainId, bytes32 _srcSenderAddress)
        external
        view
        returns (bool);
}
