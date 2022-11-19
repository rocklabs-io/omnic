// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.9;
pragma abicoder v2;

interface IOmnicNode {
    /**
     * @notice Only omnic endpoint can call it to send the message.
     * @param _srcSenderAddress     <! EVM chain user application address who call sendMessage func
     * @param _nonce                <! the outboundNonce for source chain user application
     * @param _dstChainId           <! the destination chain id, e.g. ethereum = 1,
     * @param _recipientAddress     <! the address on destination chain (using bytes32 to adapt other different address format).
     * @param _message              <! a custom bytes message to send to the destination contract
     * @param _refundAddress        <! if the source transaction is cheaper than the amount of value passed, refund the additional amount to this address
     * @param _ERC20PaymentAddress  <! using ERC20 (specified by omnic) token to pay for the transaction
     */
    function send(
        address _srcSenderAddress,
        uint64 _nonce,
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bytes calldata _message,
        address payable _refundAddress,
        address _ERC20PaymentAddress
    ) external payable;

    /**
     * @notice Only Omnic proxy canister can call it to send the message.
     * @param _message   <! a custom bytes message to send to the destination contract
     * @param _gasLimit  <! the gas limit for external contract execution
     */
    function processMessage(bytes calldata _message, uint256 _gasLimit)
        external
        returns (bool);

    // can only withdraw the receivable of the msg.sender
    function withdrawNativeFee(address payable _to, uint _amount) external;

    function withdrawERC20Fee(
        address _to,
        address _ERC20FeeToken,
        uint _amount
    ) external;

    // public interface for everyone to get some useful information
    /**
     * @notice Only omnic endpoint can call it to send the message.
     * @param _message              <! a custom bytes message to send to the destination contract
     * @param _payInERC20           <! if using ERC20 (specified by omnic) token to pay for the transaction
     */
    function estimateFees(
        bytes calldata _message,
        bool _payInERC20
    ) external view returns (uint256 nativeFee, uint256 erc20Fee);

    function getNativeTokenFee() external view returns (uint256);

    function getERC20TokenFee(address _address) external view returns (uint256);

    function getLatestRoot() external view returns (bytes32);

    function rootExists(bytes32 _root) external view returns (bool);
}
