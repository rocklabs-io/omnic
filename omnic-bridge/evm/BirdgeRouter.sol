// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.9;

// ============ Internal Imports ============
import {IBridgeWrapperToken} from "./interfaces/IBridgeWrapperToken.sol";
import {IMessageRecipient} from "./interfaces/IMessageRecipient.sol";
import {IOmnic} from "./interfaces/IOmnic.sol";
import {IXAppConnectionManager} from "./interfaces/IXAppConnectionManager.sol";
// ============ External Imports ============
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";

/**
 * @title BridgeRouter
 */
contract BridgeRouter is OwnableUpgradeable, IMessageRecipient {
    // ============ Libraries ============

    using SafeERC20 for IERC20;

    // ============ Mutable ============
    // ChainId => Birdge Router Address
    mapping(uint32 => bytes32) routers;

    // chainid => (tokenAddress => wrapperToken)
    mapping(uint32 => mapping(bytes32 => bytes32)) supportTokens;

    IXAppConnectionManager public xAppConnectionManager;

    // ============ Enums ============
    enum OperationTypes {
        Invalid,
        TokenId,
        Message,
        Transfer
    }

    // ============ Upgrade Gap ============

    // gap for upgrade safety
    uint256[49] private __GAP;

    // ======== Events =========

    event Send(
        address indexed srcChainTokenAddress,
        address indexed srcSender,
        uint32 indexed dstChainId,
        bytes32 dstRecipientAddress,
        uint256 amount
    );

    event Receive(
        uint64 indexed nonce,
        address indexed token,
        address indexed recipient,
        uint256 amount
    );

    modifier onlyOmnic() {
        require(_isOmnicContract(msg.sender), "!omnic");
        _;
    }

    modifier onlyBirdgeRouter(uint32 _chainId, bytes32 _router) {
        require(_isBirdgeRouter(_chainId, _router), "!birdge router");
        _;
    }

    // ======== Receive =======
    receive() external payable {}

    // ======== Initializer ========

    function initialize(address _xAppConnectionManager) public initializer {
        xAppConnectionManager = IXAppConnectionManager(_xAppConnectionManager);
        __Ownable_init();
    }

    // ======== External: Handle =========

    function addSupportToken(uint32 _chainId, bytes32 _tokenAddress, bytes32 _wrapperTokenAddress)
        external
        onlyOwner
        returns (bool)
    {
        supportTokens[_chainId][_tokenAddress] = _wrapperTokenAddress;
        return true;
    }

    function handleMessage(
        uint32 _srcChainId,
        bytes32 _srcSenderAddress,
        uint32 _nonce,
        bytes memory _payload
    )
        external
        override
        onlyOmnic
        onlyBirdgeRouter(_srcChainId, _srcSenderAddress)
    {
        // decode message from payload
        (
            uint32 srcChainId,
            bytes32 srcTokenAddress,
            uint32 operationType,
            bytes32 dstRecipientAddress,
            uint256 amount,
            bytes32 detailsHash
        ) = abi.decode(_payload, (uint32, bytes32, uint32, bytes32, uint256, bytes32));
        require(dstRecipientAddress != bytes32(0), "!dstRecipientAddress");
        // handle message
        require(operationType == uint32(OperationTypes.Transfer), "!transfer");
        address recipient = address(uint160(uint256(dstRecipientAddress)));
        bytes32 wrapperToken = supportTokens[srcChainId][srcTokenAddress];
        require(wrapperToken != bytes32(0), "!supported token");
        // only evm supported
        address _token = address(uint160(uint256(wrapperToken)));
        IBridgeWrapperToken(_token).mint(recipient, amount);
        IBridgeWrapperToken(_token).setDetailsHash(detailsHash);
        // emit Receive event
        emit Receive(
            _nonce,
            _token,
            recipient,
            amount
        );
    }

    // ======== External: Send Token =========

    function send(
        uint32 _srcChainId,
        bytes32 _tokenAddress,
        uint256 _amount,
        uint32 _dstChainId,
        bytes32 _recipientAddress
    ) external {
        // validate inputs
        require(_srcChainId != 0, "invalid src chain id");
        require(_recipientAddress != bytes32(0), "!recip");
        require(_amount != 0, "!amount");

        // evm address
        address tokenAddress = address(uint160(uint256(_tokenAddress)));
        IBridgeWrapperToken _t = IBridgeWrapperToken(tokenAddress);
        bytes32 _detailsHash;

        if (supportTokens[_srcChainId][_tokenAddress] != bytes32(0)) {
            // if the token originates on this chain,
            // hold the tokens in escrow in the Router
            IERC20(tokenAddress).safeTransferFrom(
                msg.sender,
                address(this),
                _amount
            );
            // query token contract for details and calculate detailsHash
            _detailsHash = keccak256(
                abi.encodePacked(
                    bytes(_t.name()).length,
                    _t.name(),
                    bytes(_t.symbol()).length,
                    _t.symbol(),
                    _t.decimals()
                )
            );
        } else {
            // if the token originates on a remote chain,
            // burn the representation tokens on this chain
            _t.burn(msg.sender, _amount);
            _detailsHash = _t.detailsHash();
        }
        // format Transfer message
        bytes memory payload = abi.encode(
            _srcChainId,
            _tokenAddress,
            OperationTypes.Transfer,
            _recipientAddress,
            _amount,
            _detailsHash
        );
        // send message to destination chain bridge router
        _sendTransferMessage(_dstChainId, payload);
        // emit Send event to record token sender
        emit Send(tokenAddress, msg.sender, _dstChainId, _recipientAddress, _amount);
    }

    // ============ Internal: Send ============

    function _isOmnicContract(address _potentialOmnicContract)
        internal
        view
        returns (bool)
    {
        return xAppConnectionManager.isOmnicContract(_potentialOmnicContract);
    }

    function _isBirdgeRouter(
        uint32 _chainId,
        bytes32 _potentialBirdgeRouterContract
    ) internal view returns (bool) {
        return
            routers[_chainId] == _potentialBirdgeRouterContract &&
            _potentialBirdgeRouterContract != bytes32(0);
    }

    function _sendTransferMessage(uint32 _destination, bytes memory _payload)
        internal
    {
        // get remote BridgeRouter address; revert if not found
        bytes32 _router = routers[_destination];
        require(_router != bytes32(0));
        // send message to destination chain
        IOmnic(xAppConnectionManager.omnic()).enqueueMessage(
            _destination,
            _router,
            _payload
        );
    }

}
