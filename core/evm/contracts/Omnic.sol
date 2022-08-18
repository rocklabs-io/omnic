// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

import {QueueLib} from "./libs/Queue.sol";
import {QueueManager} from "./Queue.sol";
import {Types} from "./libs/Types.sol";
import {IMessageRecipient} from "./interfaces/IMessageRecipient.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";
// Omnic Crosschain message passing protocol core contract

contract Omnic is Ownable, QueueManager {

    using QueueLib for QueueLib.Queue;
    // ============ Constants ============
    uint32 public immutable chainId;
    address public omnicCanisterAddr;

    // Maximum bytes per message = 2 KiB
    // (somewhat arbitrarily set to begin)
    uint256 public constant MAX_MESSAGE_BODY_BYTES = 2 * 2**10;

    // domain => next available nonce for the domain
    mapping(uint32 => uint32) public nonces;

    // re-entrancy guard
    uint8 private entered;

    event EnqueueMessage(
        bytes32 indexed messageHash,
        uint32 indexed dstNonce,
        uint32 srcChainId,
        bytes32 srcSenderAddress,
        uint32 dstChainId,
        bytes32 recipient,
        bytes data
    );

    event ProcessMessage(
        bytes32 indexed messageHash,
        uint32 dstNonce,
        uint32 srcChainId,
        bytes32 srcSenderAddress,
        uint32 dstChainId,
        bytes32 recipient,
        bytes data,
        bool indexed success,
        bytes indexed returnData
    );

    constructor() {
        chainId = uint32(block.chainid);
    }

    function initialize() public initializer {
        // initialize queue, 
        __QueueManager_initialize();
        entered = 1;
    }

    function setOmnicCanisterAddr(address addr) public onlyOwner {
        omnicCanisterAddr = addr;
    }

    function enqueueMessage(
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bytes memory payload
    ) public {
        require(payload.length <= MAX_MESSAGE_BODY_BYTES, "msg too long");
        // get the next nonce for the destination domain, then increment it
        uint32 _nonce = nonces[_dstChainId];
        nonces[_dstChainId] = _nonce + 1;
        Types.MessageFormat memory _message = Types.MessageFormat(
            chainId,
            bytes32(uint256(uint160(msg.sender))),
            _nonce,
            _dstChainId,
            _recipientAddress,
            payload
        );
        bytes32 _messageHash = keccak256(abi.encode(_message));
        queue.enqueue(_message);
        emit EnqueueMessage(
            _messageHash, 
            _nonce, 
            chainId,
            bytes32(uint256(uint160(msg.sender))),
            _dstChainId,
            _recipientAddress,
            payload
        );

    }

    // only omnic canister can call this func
    function processMessage(Types.MessageFormat memory _message) public returns (bool success){
        require(msg.sender == omnicCanisterAddr);

        require(_message._dstChainId == chainId, "!destination");
        bytes32 _messageHash = keccak256(abi.encode(_message));
        // check re-entrancy guard
        require(entered == 1, "!reentrant");
        entered = 0;

        // call handle function
        IMessageRecipient(Types.bytes32ToAddress(_message._recipientAddress)).handleMessage(
            _message._srcChainId,
            _message._srcSenderAddress,
            _message._nonce,
            _message.payload
        );
        // emit process results
        emit ProcessMessage(
            _messageHash, 
            _message._nonce,
            _message._srcChainId,
            _message._srcSenderAddress,
            _message._dstChainId,
            _message._recipientAddress,
            _message.payload,
            true, 
            ""
        );
        // reset re-entrancy guard
        entered = 1;
        // return true
        return true;
        
    }
}
