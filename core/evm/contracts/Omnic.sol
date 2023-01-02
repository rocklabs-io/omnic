// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/utils/math/SafeMath.sol";

//internal
import {QueueLib} from "./libs/Queue.sol";
import {QueueManager} from "./utils/QueueManager.sol";
import {MerkleLib} from "./libs/Merkle.sol";
import {Types} from "./libs/Types.sol";
import {TypeCasts} from "./utils/Utils.sol";
import {IOmnicReciver} from "./interfaces/IOmnicReciver.sol";
import {IOmnicFeeManager} from "./interfaces/IOmnicFeeManager.sol";
import {OmnicBase} from "./OmnicBase.sol";



// Omnic Crosschain message passing protocol core contract
contract Omnic is QueueManager, OmnicBase {
    using QueueLib for QueueLib.Queue;
    using MerkleLib for MerkleLib.Tree;
    using SafeMath for uint256;
    using SafeERC20 for IERC20;
    MerkleLib.Tree public tree;

    // re-entrancy
    uint8 internal entered;

    // ic canister which is responsible for message management, verification and update
    address public omnicProxyCanisterAddr;

    // Maximum bytes per message = 2 KiB
    // (somewhat arbitrarily set to begin)
    uint256 public constant MAX_MESSAGE_BODY_BYTES = 2 * 2**10;

    // chainId => next available nonce for the chainId
    mapping(uint32 => uint32) public nonces;

     // Token and Contracts
    IERC20 public erc20FeeToken; // choose a ERC20 token as fee token specified by omnic owner
    IOmnicFeeManager public omnicFeeManager;

    uint256 public nativeFees;
    // ERC20 token => fee amount
    mapping(address => uint256) public erc20Fees; // record different ERC20 fee amount


    // gap for upgrade safety
    uint256[49] private __GAP;

    // ============ modifiers  ============

    modifier onlyProxyCanister() {
        require(msg.sender == omnicProxyCanisterAddr, "!proxyCanisterAddress");
        _;
    }

    // ============ Events  ============
    event SendMessage(
        bytes32 indexed messageHash,
        uint256 indexed leafIndex,
        uint32 indexed dstChainId,
        uint32 nonce,
        bytes message
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
        entered = 1;
    }

    function initialize(address proxyCanisterAddr, address feeManagerAddr) public initializer {
        __QueueManager_initialize();
        __OmnicBase_initialize();
        entered = 1;
        omnicProxyCanisterAddr = proxyCanisterAddr;
        omnicFeeManager = IOmnicFeeManager(feeManagerAddr);
    }

    function sendMessage(
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bytes memory _payload,
        address payable _refundAddress,
        address _erc20PaymentAddress
    ) public payable {
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
        // refund if sent too much
        uint256 amount = msg.value.sub(nativeProtocolFee);
        if (amount > 0) {
            (bool success, ) = _refundAddress.call{value: amount}("");
            require(success, "Omnic: failed to refund");
        }

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
        bytes32 _messageHash = keccak256(_message);
        tree.insert(_messageHash);
        // enqueue the new Merkle root after inserting the message
        queue.enqueue(tree.root());

        emit SendMessage(
            _messageHash,
            tree.count - 1,
            chainId,
            _nonce,
            _message
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
            bytes memory _payload
        ) = abi.decode(
                _message,
                (uint32, bytes32, uint32, uint32, bytes32, bytes)
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

    function _handleProtocolFee(
        address _srcSenderAddress,
        address _erc20PaymentAddress,
        uint256 _msgLength
    ) internal returns (uint256 protocolNativeFee) {
        // asset fee pay with native token or ERC20 token
        bool payInNative = _erc20PaymentAddress == address(0x0) ||
            address(erc20FeeToken) == address(0x0);
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

    // ============ Public Functions  ============
    function getLatestRoot() public view returns (bytes32) {
        require(queue.length() != 0, "no item in queue");
        return queue.lastItem();
    }

    function rootExists(bytes32 _root) public view returns (bool) {
        return queue.contains(_root);
    }
}
