// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

//internal
import {QueueLib} from "./libs/Queue.sol";
import {QueueManager} from "./utils/QueueManager.sol";
import {MerkleLib} from "./libs/Merkle.sol";
import {Types} from "./libs/Types.sol";
import {TypeCasts} from "./utils/utils.sol";
import {IOmnicReciver} from "./interfaces/IOmnicReciver.sol";

//external
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";
// Omnic Crosschain message passing protocol core contract

contract Omnic is QueueManager, Ownable {

    using QueueLib for QueueLib.Queue;
    using MerkleLib for MerkleLib.Tree;
    MerkleLib.Tree public tree;

    // ============ Constants ============
    uint32 public immutable chainId;
    bytes32 public committedRoot;

    // omnic state
    enum States {  
        UnInitialized,
        Active, // contract is good
        Stopped // fraud occurs in contract
    }
    // Current state of contract
    States public state;

    // re-entrancy
    uint8 internal entered;

    // ic canister which is responsible for message management, verification and update 
    address public omnicProxyCanisterAddr;

    // Maximum bytes per message = 2 KiB
    // (somewhat arbitrarily set to begin)
    uint256 public constant MAX_MESSAGE_BODY_BYTES = 2 * 2**10;

    // chainId => next available nonce for the chainId
    mapping(uint32 => uint32) public nonces;

    // gap for upgrade safety
    uint256[49] private __GAP;

    modifier onlyProxyCanister {
        require(msg.sender == omnicProxyCanisterAddr, "!proxyCanisterAddress");
        _;
    }
    
    modifier notStopped() {
        require(state != States.Stopped, "contract stopped");
        _;
    }

    event SendMessage(
        bytes32 indexed messageHash,
        uint256 indexed leafIndex,
        uint64 indexed dstChainIdAndNonce,
        bytes options,
        bytes payload
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

    event UpdateProxyCanister(address oldProxyCanisterAddr, address newProxyCanisterAddr);

    constructor() {
        chainId = uint32(block.chainid);
    }

    function initialize(address _proxyCanisterAddr) public initializer {
        // initialize queue, 
        __QueueManager_initialize();
        omnicProxyCanisterAddr = _proxyCanisterAddr;
        entered = 1;
        state = States.Active;

    }

    function setOmnicCanisterAddr(address _newProxyCanisterAddr) public onlyOwner {
        address _oldProxyCanisterAddr = omnicProxyCanisterAddr;
        omnicProxyCanisterAddr = _newProxyCanisterAddr;
        emit UpdateProxyCanister(_oldProxyCanisterAddr, _newProxyCanisterAddr);
    }

    function sendMessage(
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bytes memory _options,
        bytes memory _payload
    ) public {
        require(_payload.length <= MAX_MESSAGE_BODY_BYTES, "msg too long");
        // get the next nonce for the destination domain, then increment it
        uint32 _nonce = nonces[_dstChainId];
        nonces[_dstChainId] = _nonce + 1;

        bytes memory _message = Types.formatMessage(
            chainId,
            TypeCasts.addressToBytes32(msg.sender),
            _nonce,
            _dstChainId,
            _recipientAddress,
            _payload
        );
        bytes32 _messageHash = keccak256(abi.encode(_message));
        tree.insert(_messageHash);
        // enqueue the new Merkle root after inserting the message
        queue.enqueue(tree.root());

        emit SendMessage(
            _messageHash,
            tree.count - 1,
            _dstChainIdAndNonce(chainId, _nonce), 
            _options,
            _payload
        );

    }

    // only omnic canister can call this func
    function processMessage(
        bytes32 _messageHash,
        uint32 _srcChainId,
        bytes32 _srcSenderAddress,
        uint32 _nonce,
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bytes memory payload
    ) public onlyProxyCanister returns (bool success) {
        require(_dstChainId == chainId, "!destination");
        // check re-entrancy guard
        require(entered == 1, "!reentrant");
        entered = 0;

        // call handle function
        IOmnicReciver(TypeCasts.bytes32ToAddress(_recipientAddress)).handleMessage(
            _srcChainId,
            _srcSenderAddress,
            _nonce,
            payload
        );
        // emit process results
        emit ProcessMessage(
            _messageHash, 
            _nonce,
            _srcChainId,
            _srcSenderAddress,
            _dstChainId,
            _recipientAddress,
            payload,
            true, 
            ""
        );
        // reset re-entrancy guard
        entered = 1;
        // return true
        return true;
    }

    function _dstChainIdAndNonce(uint32 _dstChainId, uint32 _nonce)
        internal
        pure
        returns (uint64)
    {
        return (uint64(_dstChainId) << 32) | _nonce;
    }
}
