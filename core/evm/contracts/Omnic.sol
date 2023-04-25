// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

//external
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/utils/math/SafeMath.sol";
import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {Initializable} from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";


//internal
import {TypeCasts} from "./utils/Utils.sol";
import {IOmnic} from "./interfaces/IOmnic.sol";
import {IOmnicReciver} from "./interfaces/IOmnicReciver.sol";
import {IOmnicFeeManager} from "./interfaces/IOmnicFeeManager.sol";

// Omnic Crosschain message passing protocol core contract
contract Omnic is IOmnic, Initializable, OwnableUpgradeable {
    using SafeMath for uint256;
    using SafeERC20 for IERC20;

    // ================================ Variables ===================================
    // Maximum bytes per message = 2 KiB
    // (somewhat arbitrarily set to begin)
    uint256 public constant MAX_MESSAGE_BODY_BYTES = 2 * 2**10;
    uint32 public chainId;
    // ic canister which is responsible for message management, verification and update
    address public omnicProxyCanisterAddr;
     // Token and Contracts
    IOmnicFeeManager public omnicFeeManager;
    // Token and Contracts
    IERC20 public erc20FeeToken; // choose a ERC20 token as fee token specified by omnic owner
    // ERC20 token => fee amount
    mapping(address => uint256) public erc20Fees; // record different ERC20 fee amount
    uint256 public nativeFees;

    mapping(address => bool) public whitelisted; // whitelisted addresses can send msg for free

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

    modifier onlyProxyCanister() {
        require(msg.sender == omnicProxyCanisterAddr, "!proxyCanisterAddress");
        _;
    }

    // ============ Events  ============
    event SendMessage(
        bytes32 indexed messageHash,
        bytes message,
        uint32 indexed srcChainId,
        address sender,
        uint32 indexed destChainId,
        bytes32 receiptAddress
    );

    event ProcessMessage(
        bytes32 indexed messageHash,
        bytes indexed returnData,
        bool success
    );

    event InvalidTransaction(
        uint32 indexed srcChainId,
        bytes32 srcAddress,
        address indexed dstAddress,
        uint64 nonce,
        bytes32 messageHash
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

    event UpdateProxyCanister(
        address oldProxyCanisterAddr,
        address newProxyCanisterAddr
    );

    // ============== Start ===============
    constructor() {}

    function initialize(address proxyCanisterAddr, address feeManagerAddr) public initializer {
        __Ownable_init();
        chainId = uint32(block.chainid); 
        omnicProxyCanisterAddr = proxyCanisterAddr;
        omnicFeeManager = IOmnicFeeManager(feeManagerAddr);
    }

    function sendMessage(
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bytes memory _payload,
        address payable _refundAddress,
        address _erc20PaymentAddress
    ) external payable override sendNonReentrant {
        require(_payload.length <= MAX_MESSAGE_BODY_BYTES, "msg too long");
        // compute all the fees
        uint256 nativeProtocolFee = _handleProtocolFee(
            msg.sender,
            _erc20PaymentAddress,
            _payload.length
        );

        // assert whether the user has enough native token amount
        require(
            nativeProtocolFee <= msg.value,
            "Omnic: not enough value for fees"
        );

        // get the next nonce for the destination domain, then increment it
        uint64 _nonce = ++outboundNonce[_dstChainId][msg.sender];

        Message memory m = Message (
            IOmnicReciver.MessageType.SYN,
            chainId,
            TypeCasts.addressToBytes32(msg.sender),
            _nonce,
            _dstChainId,
            _recipientAddress,
            _payload
        );
        bytes memory _message = abi.encode(m);
        bytes32 _messageHash = keccak256(_message);

        emit SendMessage(
            _messageHash,
            _message,
            chainId,
            msg.sender,
            _dstChainId,
            _recipientAddress
        );

        // refund if sent too much
        uint256 amount = msg.value.sub(nativeProtocolFee);
        if (amount > 0) {
            (bool success, ) = _refundAddress.call{value: amount}("");
            require(success, "Omnic: failed to refund");
        }
    }

    // fee free function for whitelisted contracts
    // for omnic bridge's CreatePool, Add/RemoveLiquidity operations
    function sendMessageFree(
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bytes memory _payload
    ) external override sendNonReentrant {
        require(whitelisted[msg.sender], "not whitelisted caller");
        require(_payload.length <= MAX_MESSAGE_BODY_BYTES, "msg too long");
        // get the next nonce for the destination domain, then increment it
        // get the next nonce for the destination domain, then increment it
        uint64 _nonce = ++outboundNonce[_dstChainId][msg.sender];

        Message memory m = Message (
            IOmnicReciver.MessageType.SYN,
            chainId,
            TypeCasts.addressToBytes32(msg.sender),
            _nonce,
            _dstChainId,
            _recipientAddress,
            _payload
        );
        bytes memory _message = abi.encode(m);
        bytes32 _messageHash = keccak256(_message);

        emit SendMessage(
            _messageHash,
            _message,
            chainId,
            msg.sender,
            _dstChainId,
            _recipientAddress
        );
    }

    // ==================================== Internal Func =======================================

    function _isContract(address addr) internal view returns (bool) {
        uint size;
        assembly {
            size := extcodesize(addr)
        }
        return size != 0;
    }

    function _processMessage(Message memory m) internal returns(bool) {
        require(m.dstChainId == chainId, "!destination");
        require(
            m.payload.length <= MAX_MESSAGE_BODY_BYTES,
            "Omnic: message too long"
        );
        bytes32 _messageHash = keccak256(m.payload);

        // if the dst is not a contract, then emit and return early. This will break inbound nonces, but this particular
        // path is already broken and wont ever be able to deliver anyways
        address dstAddress = TypeCasts.bytes32ToAddress(m.recipient);
        if (!_isContract(dstAddress)) {
            emit InvalidTransaction(
                m.srcChainId,
                m.srcSenderAddress,
                dstAddress,
                m.nonce,
                _messageHash
            );
            return false;
        }
       require(
            m.nonce == ++inboundNonce[m.srcChainId][m.srcSenderAddress],
            "Omnic: wrong nonce"
        );

        // cache failed message
        CacheMessage storage cache = cacheMessage[m.srcChainId][
            m.srcSenderAddress
        ];
        require(
            cache.msgHash == bytes32(0),
            "Omnic: in message blocking"
        );

        try
            IOmnicReciver(dstAddress).handleMessage(
                m.t,
                m.srcChainId,
                m.srcSenderAddress,
                m.nonce,
                m.payload
            )
        {
            // emit process results
            emit ProcessMessage(_messageHash, "", true);
            return true;
        } catch (bytes memory e) {
            // revert nonce if any uncaught errors/exceptions if the ua chooses the blocking mode
            cacheMessage[m.srcChainId][m.srcSenderAddress] = CacheMessage(
                uint64(m.payload.length),
                dstAddress,
                keccak256(m.payload)
            );
            emit SetCacheMessage(
                m.srcChainId,
                m.srcSenderAddress,
                dstAddress,
                m.nonce,
                m.payload,
                e
            );
            return false;
        }
    }

    // only omnic canister can call this func
    function processMessage(bytes memory _message)
        external override
        onlyProxyCanister
        receiveNonReentrant
        returns (bool success)
    {
        // decode message
        Message memory m = abi.decode(
                _message,
                (Message)
            );

        return _processMessage(m);
    }

    function processMessageBatch(bytes[] memory _messages)
        external override
        onlyProxyCanister
        receiveNonReentrant
        returns (bool success)
    {
        for(uint i =0; i < _messages.length; i++){
            Message memory m = abi.decode(
                _messages[i],
                (Message)
            );
            _processMessage(m);
        }
        return true;
    }

    function retryProcessMessage(
        IOmnicReciver.MessageType t,
        uint32 _srcChainId,
        bytes32 _srcSenderAddress,
        bytes calldata _message
    ) external override receiveNonReentrant {
        CacheMessage storage cache = cacheMessage[_srcChainId][
            _srcSenderAddress
        ];
        require(
            cache.msgHash != bytes32(0),
            "Omnic: no stored payload"
        );
        require(
            _message.length == cache.msgLength &&
                keccak256(_message) == cache.msgHash,
            "Omnic: invalid payload"
        );

        address dstAddress = cache.dstAddress;
        // empty the cacheMessage
        cache.msgLength = 0;
        cache.dstAddress = address(0);
        cache.msgHash = bytes32(0);

        uint64 nonce = inboundNonce[_srcChainId][_srcSenderAddress];

        IOmnicReciver(dstAddress).handleMessage(
            t,
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
            "Omnic: no stored payload"
        );
        require(
            cache.dstAddress == msg.sender,
            "Omnic: invalid caller"
        );

        // empty the cacheMessage
        cache.msgLength = 0;
        cache.dstAddress = address(0);
        cache.msgHash = bytes32(0);

        // emit the event with the new nonce
        emit ForceResumeReceive(_srcChainId, _srcSenderAddress);
    }

    // ///////////////////
    function _handleProtocolFee(
        address _srcSenderAddress,
        address _erc20PaymentAddress,
        uint256 _msgLength
    ) internal returns (uint256 protocolNativeFee) {
        // asset fee pay with native token or ERC20 token
        bool payInNative = _erc20PaymentAddress == address(0x0);
        uint256 protocolFee = omnicFeeManager.getFees(!payInNative, _msgLength);

        if (protocolFee > 0) {
            if (payInNative) {
                nativeFees = nativeFees.add(protocolFee);
                protocolNativeFee = protocolFee;
            } else {
                require(
                    _erc20PaymentAddress == _srcSenderAddress ||
                        _erc20PaymentAddress == tx.origin,
                    "Omnic: must be paid by sender or origin"
                );
                if(protocolFee == 0) {
                    return protocolNativeFee;
                }
                // transfer the fee with ERC20 token
                erc20FeeToken.safeTransferFrom(
                    _erc20PaymentAddress,
                    address(this),
                    protocolFee
                );
                
                address erc20TokenAddr = address(erc20FeeToken);
                erc20Fees[erc20TokenAddr] = erc20Fees[erc20TokenAddr].add(
                    protocolFee
                );
            }
        }
    }

    function setWhitelist(address addr, bool value) external onlyOwner {
        whitelisted[addr] = value;
    }

    // withdraw ERC20 token function
    function withdrawERC20Fee(
        address _to,
        address _erc20FeeToken,
        uint256 _amount
    ) external onlyOwner {
        require(
            erc20Fees[_erc20FeeToken] >= _amount,
            "Omnic: insufficient ERC20 amount"
        );
        erc20Fees[_erc20FeeToken] = erc20Fees[_erc20FeeToken].sub(_amount);
        IERC20(_erc20FeeToken).safeTransfer(_to, _amount);
    }

    // withdraw native token function.
    function withdrawNativeFee(address payable _to, uint256 _amount)
        external
        onlyOwner
    {
        require(_to != address(0x0), "Omnic: _to cannot be zero address");
        nativeFees = nativeFees.sub(_amount);

        (bool success, ) = _to.call{value: _amount}("");
        require(success, "Omnic: withdraw failed");
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
}
