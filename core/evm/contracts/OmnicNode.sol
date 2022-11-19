// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.9;
pragma abicoder v2;

//internal
import {QueueLib} from "./libs/Queue.sol";
import {QueueManager} from "./utils/QueueManager.sol";
import {MerkleLib} from "./libs/Merkle.sol";
import {Message} from "./libs/Message.sol";
import {TypeCasts} from "./utils/Utils.sol";
import {IOmnicNode} from "./interfaces/IOmnicNode.sol";
import {IOmnicReciver} from "./interfaces/IOmnicReciver.sol";
import {IOmnicFeeManage} from "./interfaces/IOmnicFeeManage.sol";
import {IOmnicEndpoint} from "./interfaces/IOmnicEndpoint.sol";
import {VersionManage} from "./VersionManage.sol";

//external
import "@openzeppelin/contracts/utils/math/SafeMath.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";

contract OmnicNode is
    QueueManager,
    IOmnicNode,
    VersionManage,
    ReentrancyGuard,
    OwnableUpgradeable
{
    using SafeERC20 for IERC20;
    using SafeMath for uint256;

    using QueueLib for QueueLib.Queue;
    using MerkleLib for MerkleLib.Tree;
    MerkleLib.Tree public tree;

    // ic canister which is responsible for message management, verification and update
    address public omnicProxyCanisterAddr;

    // Maximum bytes per message = 2 KiB
    // (somewhat arbitrarily set to begin)
    uint256 public constant MAX_MESSAGE_BODY_BYTES = 2 * 2**10;

    IOmnicEndpoint public immutable endpoint;

    // Token and Contracts
    IERC20 public erc20FeeToken; // choose a ERC20 token as fee token specified by omnic owner
    IOmnicFeeManage public omnicFeeManage;

    uint256 public nativeFees;
    // ERC20 token => fee amount
    mapping(address => uint256) public erc20Fees; // record different ERC20 fee amount

    mapping(uint32 => bytes32) public nodeLookup; // remote omnic nodes

    // gap for upgrade safety
    uint256[49] private __GAP;

    // ==================================== Modifiers  ===========================================

    modifier onlyEndpoint() {
        require(address(endpoint) == msg.sender, "OmnicNode: only endpoint");
        _;
    }

    modifier onlyProxyCanister() {
        require(
            msg.sender == omnicProxyCanisterAddr,
            "OmnicNode: !proxyCanisterAddress"
        );
        _;
    }

    // ===================== Events ========================
    event SendMessage(bytes message);
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
    event WithdrawERC20Fee(
        address feeManage,
        address indexed to,
        address erc20Token,
        uint256 amount
    );
    event WithdrawNativeFee(
        address feeManage,
        address indexed to,
        uint256 amount
    );
    event UpdateProxyCanister(
        address oldProxyCanisterAddr,
        address newProxyCanisterAdd
    );
    event SetERC20FeeToken(address indexed tokenAddress);
    event SetRemoteNode(uint32 indexed chainId, bytes32 node);
    event SetFeeManage(address indexed feeManageAddress);

    // ====================================== Start ======================================

    constructor(address _endpoint) {
        require(
            _endpoint != address(0x0),
            "OmnicNode: endpoint cannot be zero address"
        );
        endpoint = IOmnicEndpoint(_endpoint);
    }

    function initialize(address _proxyCanisterAddr) public initializer {
        require(
            _proxyCanisterAddr != address(0x0),
            "OmnicNode: proxy cannot be zero address"
        );
        __QueueManager_initialize();
        omnicProxyCanisterAddr = _proxyCanisterAddr;
    }

    // ====================================  Proxy Canister Call ====================================

    function processMessage(bytes memory _message, uint256 _gasLimit)
        external
        override
        onlyProxyCanister
        returns (bool)
    {
        require(
            _message.length <= MAX_MESSAGE_BODY_BYTES,
            "OmnicNode: message too long"
        );
        Message.Packet memory packet;
        // decode message
        packet = Message.unpacketMessage(_message);

        // packet content assertion
        require(
            packet.dstChainId == endpoint.getChainId(),
            "OmnicNode: invalid dstChain Id"
        );

        // if the dst is not a contract, then emit and return early. This will break inbound nonces, but this particular
        // path is already broken and wont ever be able to deliver anyways
        address dstAddress = TypeCasts.bytes32ToAddress(packet.recipientAddress);
        if (!_isContract(dstAddress)) {
            emit InvalidTransaction(
                packet.srcChainId,
                packet.srcAddress,
                dstAddress,
                packet.nonce,
                keccak256(packet.messageBody)
            );
            return false;
        }
        // call endpoint to handle the message
        endpoint.processMessage(
            packet.srcChainId,
            packet.srcAddress,
            dstAddress,
            packet.nonce,
            _gasLimit,
            packet.messageBody
        );
        emit ProcessMessage(keccak256(_message), "", true);
        return true;
    }

    // ====================================  Endpoint Call ====================================

    function send(
        address _srcSenderAddress,
        uint64 _nonce,
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bytes calldata _message,
        address payable _refundAddress,
        address _erc20PaymentAddress
    ) external payable override onlyEndpoint {
        uint32 dstChainId = _dstChainId;
        bytes32 recipientAddress = _recipientAddress;
        bytes memory message = _message;
        uint64 nonce = _nonce;
        require(
            nodeLookup[dstChainId] != bytes32(0),
            "OmnicNode: no omnic node deployed on dstChainId."
        );

        // compute all the fees
        uint256 nativeProtocolFee = _handleProtocolFee(
            _srcSenderAddress,
            _erc20PaymentAddress,
            _message.length
        );

        // assert whether the user has enough native token amount
        require(
            nativeProtocolFee <= msg.value,
            "OmnicNode: not enough native for fees"
        );
        // refund if they send too much
        uint256 amount = msg.value.sub(nativeProtocolFee);
        if (amount > 0) {
            (bool success, ) = _refundAddress.call{value: amount}("");
            require(success, "OmnicNode: failed to refund");
        }

        // emit the data packet
        bytes32 senderAddress = TypeCasts.addressToBytes32(_srcSenderAddress);
        bytes memory encodedMessage = Message.formatMessage(
            endpoint.getChainId(),
            senderAddress,
            nonce,
            dstChainId,
            recipientAddress,
            message
        );

        bytes32 messageHash = keccak256(encodedMessage);
        tree.insert(messageHash);
        // enqueue the new Merkle root after inserting the message
        queue.enqueue(tree.root());
        emit SendMessage(encodedMessage);
    }

    function _handleProtocolFee(
        address _srcSenderAddress,
        address _erc20PaymentAddress,
        uint256 _msgLength
    ) internal returns (uint256 protocolNativeFee) {
        // asset fee pay with native token or ERC20 token
        bool payInNative = _erc20PaymentAddress == address(0x0) ||
            address(erc20FeeToken) == address(0x0);
        uint256 protocolFee = omnicFeeManage.getFees(!payInNative, _msgLength);

        if (protocolFee > 0) {
            if (payInNative) {
                nativeFees = nativeFees.add(protocolFee);
                protocolNativeFee = protocolFee;
            } else {
                require(
                    _erc20PaymentAddress == _srcSenderAddress ||
                        _erc20PaymentAddress == tx.origin,
                    "OmnicNode: must be paid by sender or origin"
                );

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

    // ====================================  Claim Fees (Fee Manage Contract Call) ====================================

    // withdraw ERC20 token function
    function withdrawERC20Fee(
        address _to,
        address _erc20FeeToken,
        uint256 _amount
    ) external override nonReentrant {
        require(
            msg.sender == address(omnicFeeManage),
            "OmnicNode: only treasury"
        );
        require(
            erc20Fees[_erc20FeeToken] >= _amount,
            "OmnicNode: insufficient ERC20 amount"
        );
        erc20Fees[_erc20FeeToken] = erc20Fees[_erc20FeeToken].sub(_amount);
        IERC20(_erc20FeeToken).safeTransfer(_to, _amount);
        emit WithdrawERC20Fee(msg.sender, _erc20FeeToken, _to, _amount);
    }

    // withdraw native token function.
    function withdrawNativeFee(address payable _to, uint256 _amount)
        external
        override
        nonReentrant
    {
        require(_to != address(0x0), "OmnicNode: _to cannot be zero address");
        nativeFees = nativeFees.sub(_amount);

        (bool success, ) = _to.call{value: _amount}("");
        require(success, "OmnicNode: withdraw failed");
        emit WithdrawNativeFee(msg.sender, _to, _amount);
    }

    // ==================================== Internal Func =======================================

    function _isContract(address addr) internal view returns (bool) {
        uint size;
        assembly {
            size := extcodesize(addr)
        }
        return size != 0;
    }

    // ===============================  Common Sets (Owner Call) ===============================

    function setOmnicCanisterAddr(address _newProxyCanisterAddr)
        external
        onlyOwner
    {
        address _oldProxyCanisterAddr = omnicProxyCanisterAddr;
        omnicProxyCanisterAddr = _newProxyCanisterAddr;
        emit UpdateProxyCanister(_oldProxyCanisterAddr, _newProxyCanisterAddr);
    }

    function setERC20FeeToken(address _erc20FeeToken) external onlyOwner {
        require(
            _erc20FeeToken != address(0x0),
            "OmnicNode: _erc20FeeToken cannot be zero address"
        );
        erc20FeeToken = IERC20(_erc20FeeToken);
        emit SetERC20FeeToken(_erc20FeeToken);
    }

    function SetFeeManageAddr(address _feeManage) external onlyOwner {
        require(
            _feeManage != address(0x0),
            "OmnicNode: fee Manage cannot be zero address"
        );
        omnicFeeManage = IOmnicFeeManage(_feeManage);
        emit SetFeeManage(_feeManage);
    }

    function setRemoteNode(uint32 _remoteChainId, bytes32 _remoteNode)
        external
        onlyOwner
    {
        require(
            nodeLookup[_remoteChainId] == bytes32(0),
            "OmnicNode: remote node already exists"
        );
        nodeLookup[_remoteChainId] = _remoteNode;
        emit SetRemoteNode(_remoteChainId, _remoteNode);
    }

    // ==================================== Public Gets =========================================

    // returns the native fee the UA pays to cover fees
    function estimateFees(
        bytes calldata _message,
        bool _payInERC20
    ) external view override returns (uint256 nativeFee, uint256 erc20Fee) {
        // get Fee
        uint256 protocolFee = omnicFeeManage.getFees(
            _payInERC20,
            _message.length
        );
        _payInERC20 ? erc20Fee = protocolFee : nativeFee = protocolFee;
    }

    function getNativeTokenFee() external view override returns (uint256) {
        return nativeFees;
    }

    function getERC20TokenFee(address _address)
        external
        view
        override
        returns (uint256)
    {
        return erc20Fees[_address];
    }

    function getLatestRoot() external view override returns (bytes32) {
        require(queue.length() != 0, "no item in queue");
        return queue.lastItem();
    }

    function rootExists(bytes32 _root) external view override returns (bool) {
        return queue.contains(_root);
    }
}
