// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

//internal
import {QueueLib} from "./libs/Queue.sol";
import {QueueManager} from "./utils/QueueManager.sol";
import {MerkleLib} from "./libs/Merkle.sol";
import {Types} from "./libs/Types.sol";
import {TypeCasts} from "./utils/Utils.sol";
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

    // ============ modifiers  ============

    modifier onlyProxyCanister() {
        require(msg.sender == omnicProxyCanisterAddr, "!proxyCanisterAddress");
        _;
    }

    modifier notStopped() {
        require(state != States.Stopped, "contract stopped");
        _;
    }

    // ============ Events  ============
    event SendMessage(
        bytes32 indexed messageHash,
        uint256 indexed leafIndex,
        uint32 indexed dstChainId,
        uint32 nonce,
        bool waitOptimisitc,
        bytes payload
    );

    event ProcessMessage(
        bytes32 indexed messageHash,
        bytes indexed returnData,
        bool success
    );

    event UpdateProxyCanister(
        address oldProxyCanisterAddr,
        address newProxyCanisterAddr
    );

    // ============== Start ===============
    constructor() {
        chainId = uint32(block.chainid);
    }

    function initialize(address _proxyCanisterAddr)
        public
        initializer
    {
        // initialize queue,
        __QueueManager_initialize();
        omnicProxyCanisterAddr = _proxyCanisterAddr;
        entered = 1;
        state = States.Active;
    }

    function sendMessage(
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bool _waitOptimistic, // customized
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
            _waitOptimistic,
            _payload
        );
        bytes32 _messageHash = keccak256(_message);
        tree.insert(_messageHash);
        // enqueue the new Merkle root after inserting the message
        queue.enqueue(tree.root());

        emit SendMessage(
            _messageHash,
            tree.count - 1,
            chainId,
            _nonce,
            _payload
        );
    }

    // only omnic canister can call this func
    function processMessage(bytes memory _message)
        public
        onlyProxyCanister
        returns (bool success)
    {
        // decode message
        (
            uint32 _srcChainId,
            bytes32 _srcSenderAddress,
            uint32 _nonce,
            uint32 _dstChainId,
            bytes32 _recipientAddress,
            bool _waitOptimistic,
            bytes memory _payload
        ) = abi.decode(
                _message,
                (uint32, bytes32, uint32, uint32, bytes32, bool, bytes)
            );
        bytes32 _messageHash = keccak256(_message);
        require(_dstChainId == chainId, "!destination");
        // check re-entrancy guard
        require(entered == 1, "!reentrant");
        entered = 0;

        // call handle function
        IOmnicReciver(TypeCasts.bytes32ToAddress(_recipientAddress))
            .handleMessage(_srcChainId, _srcSenderAddress, _nonce, _payload);
        // emit process results
        emit ProcessMessage(_messageHash, "", true);
        // reset re-entrancy guard
        entered = 1;
        // return true
        return true;
    }

    // ============ onlyOwner Set Functions  ============
    function setOmnicCanisterAddr(address _newProxyCanisterAddr)
        public
        onlyOwner
    {
        address _oldProxyCanisterAddr = omnicProxyCanisterAddr;
        omnicProxyCanisterAddr = _newProxyCanisterAddr;
        emit UpdateProxyCanister(_oldProxyCanisterAddr, _newProxyCanisterAddr);
    }

    // ============ Public Functions  ============
    function getLatestRoot() public view returns (bytes32) {
        require(queue.length() != 0, "no item in queue");
        return queue.lastItem();
    }

    function rootExists(bytes32 _root) public view returns (bool) {
        return queue.contains(_root);
    }
}
