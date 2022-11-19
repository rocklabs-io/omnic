// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

//internal
import {IOmnicReciver} from "./interfaces/IOmnicReciver.sol";
import {IOmnicEndpoint} from "./interfaces/IOmnicEndpoint.sol";
import {IOmnicNode} from "./interfaces/IOmnicNode.sol";

//external
import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {Initializable} from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";

// Omnic Crosschain message passing protocol core contract
contract OmnicEndpoint is Initializable, OwnableUpgradeable, IOmnicEndpoint {

    // define CacheMessage to store failed transaction
    struct CacheMessage {
        uint64 msgLength;
        address dstAddress;
        bytes32 msgHash;
    }

    // ================================ Variables ===================================

    uint32 public chainId;

    /** @notice Use only one node for endpoint. 
                We'll support multi nodes and users could setup specified version node. */
    IOmnicNode public omnicNodeAddr;

    // inboundNonce = [srcChainId][srcAddress].
    mapping(uint32 => mapping(bytes32 => uint64)) public inboundNonce;
    // outboundNonce = [dstChainId][srcAddress].
    mapping(uint32 => mapping(address => uint64)) public outboundNonce;
    // CacheMessage = [srcChainId][srcAddress]
    mapping(uint32 => mapping(bytes32 => CacheMessage)) public cacheMessage;

    // gap for upgrade safety
    uint256[49] private __GAP;

    // ============================== Modifiers ====================================

    // send and receive nonreentrant lock
    uint8 internal constant _UN_LOCKED = 1;
    uint8 internal constant _LOCKED = 2;
    uint8 internal _sendEnteredState = 1;
    uint8 internal _processEnteredState = 1;

    modifier sendNonReentrant() {
        require(
            _sendEnteredState == _UN_LOCKED,
            "OmnicEndpoint: no send reentrancy"
        );
        _sendEnteredState = _LOCKED;
        _;
        _sendEnteredState = _UN_LOCKED;
    }
    modifier receiveNonReentrant() {
        require(
            _processEnteredState == _UN_LOCKED,
            "OmnicEndpoint: no receive reentrancy"
        );
        _processEnteredState = _LOCKED;
        _;
        _processEnteredState = _UN_LOCKED;
    }

    modifier onlyOmnicNode() {
        require(msg.sender == address(omnicNodeAddr), "OmnicEndpoint: !omnicNodeAddr");
        _;
    }

    // ============================== Events  =====================================

    event UpdateOmnicNodeAddr(
        address oldOmnicNodeAddr,
        address newOmnicNodeAddr
    );

    event SetCacheMessage(
        uint32 srcChainId,
        bytes32 srcAddress,
        address dstAddress,
        uint64 nonce,
        bytes message,
        bytes exceptions
    );

    event CacheClean(
        uint32 srcChainId,
        bytes32 srcAddress,
        address dstAddress,
        uint64 nonce
    );

    event ForceResumeReceive(uint32 chainId, bytes32 srcAddress);

    // ====================================== Start ======================================

    constructor() {}

    function initialize(address _omnicNodeAddr) public initializer {
        __Ownable_init();
        chainId = uint32(block.chainid);
        omnicNodeAddr = IOmnicNode(_omnicNodeAddr);
    }

    //-============================= User Application Call ===============================

    // user application call this function to send message through the omnic node.
    function sendMessage(
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bytes calldata _message,
        address payable _refundAddress,
        address _ERC20PaymentAddress
    ) external payable override sendNonReentrant {
        uint64 nonce = ++outboundNonce[_dstChainId][msg.sender];
        // call omnic node send func to send message
        _getOmnicNode().send{value: msg.value}(
            msg.sender,
            nonce,
            _dstChainId,
            _recipientAddress,
            _message,
            _refundAddress,
            _ERC20PaymentAddress
        );
    }

    //-============================ Omnic Node Call =====================================

    function processMessage(
        uint32 _srcChainId,
        bytes32 _srcSenderAddress,
        address _dstAddress,
        uint64 _nonce,
        uint256 _gasLimit,
        bytes calldata _message
    ) external override receiveNonReentrant onlyOmnicNode {
        // assert and increment the nonce.
        require(
            _nonce == ++inboundNonce[_srcChainId][_srcSenderAddress],
            "OmnicEndpoint: wrong nonce"
        );

        // cache failed message
        CacheMessage storage cache = cacheMessage[_srcChainId][
            _srcSenderAddress
        ];
        require(
            cache.msgHash == bytes32(0),
            "OmnicEndpoint: in message blocking"
        );

        try
            IOmnicReciver(_dstAddress).handleMessage{gas: _gasLimit}(
                _srcChainId,
                _srcSenderAddress,
                _nonce,
                _message
            )
        {
            // success, do nothing
        } catch (bytes memory e) {
            // revert nonce if any uncaught errors/exceptions if the ua chooses the blocking mode
            cacheMessage[_srcChainId][_srcSenderAddress] = CacheMessage(
                uint64(_message.length),
                _dstAddress,
                keccak256(_message)
            );
            emit SetCacheMessage(
                _srcChainId,
                _srcSenderAddress,
                _dstAddress,
                _nonce,
                _message,
                e
            );
        }
    }

    function retryProcessMessage(
        uint32 _srcChainId,
        bytes32 _srcSenderAddress,
        bytes calldata _message
    ) external override receiveNonReentrant {
        CacheMessage storage cache = cacheMessage[_srcChainId][
            _srcSenderAddress
        ];
        require(
            cache.msgHash != bytes32(0),
            "OmnicEndpoint: no stored payload"
        );
        require(
            _message.length == cache.msgLength &&
                keccak256(_message) == cache.msgHash,
            "OmnicEndpoint: invalid payload"
        );

        address dstAddress = cache.dstAddress;
        // empty the cacheMessage
        cache.msgLength = 0;
        cache.dstAddress = address(0);
        cache.msgHash = bytes32(0);

        uint64 nonce = inboundNonce[_srcChainId][_srcSenderAddress];

        IOmnicReciver(dstAddress).handleMessage(
            _srcChainId,
            _srcSenderAddress,
            nonce,
            _message
        );
        emit CacheClean(_srcChainId, _srcSenderAddress, dstAddress, nonce);
    }

    function forceResumeReceive(uint32 _srcChainId, bytes32 _srcSenderAddress)
        external
        override
    {
        CacheMessage storage cache = cacheMessage[_srcChainId][
            _srcSenderAddress
        ];
        // revert if no messages are cached
        require(
            cache.msgHash != bytes32(0),
            "OmnicEndpoint: no stored payload"
        );
        require(
            cache.dstAddress == msg.sender,
            "OmnicEndpoint: invalid caller"
        );

        // empty the cacheMessage
        cache.msgLength = 0;
        cache.dstAddress = address(0);
        cache.msgHash = bytes32(0);

        // emit the event with the new nonce
        emit ForceResumeReceive(_srcChainId, _srcSenderAddress);
    }

    // ============================ Only owner Set Functions  =============================

    function setOmnicCanisterAddr(address _newomnicNodeAddr) public onlyOwner {
        address _oldomnicNodeAddr = address(omnicNodeAddr);
        omnicNodeAddr = IOmnicNode(_newomnicNodeAddr);
        emit UpdateOmnicNodeAddr(_oldomnicNodeAddr, _newomnicNodeAddr);
    }

    //  ============================ Internal functions  ==================================

    function _getOmnicNode() internal view returns (IOmnicNode) {
        require(address(omnicNodeAddr) != address(0x0), "OmnicEndpoint: no omnic node address");
        return omnicNodeAddr;
    }

    // ===================== Public Functions  =========================================

    function estimateFees(
        bytes calldata _message,
        bool _payInERC20
    ) external view override returns (uint256 nativeFee, uint256 erc20Fee) {
        return
            _getOmnicNode().estimateFees(
                _message,
                _payInERC20
            );
    }

    function getInboundNonce(uint32 _srcChainId, bytes32 _srcSenderAddress)
        external
        view
        override
        returns (uint64)
    {
        return inboundNonce[_srcChainId][_srcSenderAddress];
    }

    function getOutboundNonce(uint32 _dstChainId, address _srcSenderAddress)
        external
        view
        override
        returns (uint64)
    {
        return outboundNonce[_dstChainId][_srcSenderAddress];
    }

    function getChainId() external view override returns (uint32) {
        return chainId;
    }

    function getOmnicNode() external view override returns (address) {
        return address(omnicNodeAddr);
    }

    function isSendingMessage() external view override returns (bool) {
        return _sendEnteredState == _LOCKED;
    }

    function isProcessingMessage() external view override returns (bool) {
        return _processEnteredState == _LOCKED;
    }

    function hasCacheMessage(uint32 _srcChainId, bytes32 _srcSenderAddress)
        external
        view
        override
        returns (bool)
    {
        CacheMessage storage cache = cacheMessage[_srcChainId][_srcSenderAddress];
        return cache.msgHash != bytes32(0);
    }
}
