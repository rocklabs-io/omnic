// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.9;

// ============ Internal Imports ============
import {BirdgeManager} from "./utils/BirdgeManager.sol";
import {IBridgeWrapperToken} from "./interfaces/IBridgeWrapperToken.sol";
import {WrapperERC20} from "./WrapperERC20.sol";

// ============ External Imports ============
import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";

contract BridgeWrapperToken is IBridgeWrapperToken, OwnableUpgradeable, WrapperERC20 {
    // ============ Immutables ============

    // Immutables used in EIP 712 structured data hashing & signing
    // https://eips.ethereum.org/EIPS/eip-712
    bytes32 public immutable _PERMIT_TYPEHASH =
        keccak256(
            "Permit(address owner,address spender,uint256 value,uint256 nonce,uint256 deadline)"
        );
    bytes32 private immutable _EIP712_STRUCTURED_DATA_VERSION =
        keccak256(bytes("1"));
    uint16 private immutable _EIP712_PREFIX_AND_VERSION = uint16(0x1901);

    // ============ Public Storage ============

    mapping(address => uint256) public nonces;
    /// @dev hash commitment to the name/symbol/decimals
    bytes32 public override detailsHash;

    // ============ Upgrade Gap ============

    uint256[48] private __GAP; // gap for upgrade safety

    // ============ Initializer ============

    function initialize() public override initializer {
        __Ownable_init();
    }

    // ============ Events ============

    event UpdateDetails(
        string indexed name,
        string indexed symbol,
        uint8 indexed decimals
    );

    // ============ External Functions ============

    /**
     * @notice Destroys `_amnt` tokens from `_from`, reducing the
     * total supply.
     * @dev Emits a {Transfer} event with `to` set to the zero address.
     * Requirements:
     * - `_from` cannot be the zero address.
     * - `_from` must have at least `_amnt` tokens.
     * @param _from The address from which to destroy the tokens
     * @param _amnt The amount of tokens to be destroyed
     */
    function burn(address _from, uint256 _amnt) external override onlyOwner {
        _burn(_from, _amnt);
    }

    /** @notice Creates `_amnt` tokens and assigns them to `_to`, increasing
     * the total supply.
     * @dev Emits a {Transfer} event with `from` set to the zero address.
     * Requirements:
     * - `to` cannot be the zero address.
     * @param _to The destination address
     * @param _amnt The amount of tokens to be minted
     */
    function mint(address _to, uint256 _amnt) external override onlyOwner {
        _mint(_to, _amnt);
    }

    /** @notice allows the owner to set the details hash commitment.
     * @param _detailsHash the new details hash.
     */
    function setDetailsHash(bytes32 _detailsHash) external override onlyOwner {
        if (detailsHash != _detailsHash) {
            detailsHash = _detailsHash;
        }
    }

    /**
     * @notice Set the details of a token
     * @param _newName The new name
     * @param _newSymbol The new symbol
     * @param _newDecimals The new decimals
     */
    function setDetails(
        string calldata _newName,
        string calldata _newSymbol,
        uint8 _newDecimals
    ) external override {
        bool _isFirstDetails = bytes(token.name).length == 0;
        // 0 case is the initial deploy. We allow the deploying registry to set
        // these once. After the first transfer is made, detailsHash will be
        // set, allowing anyone to supply correct name/symbols/decimals
        require(
            _isFirstDetails ||
                BirdgeManager.getDetailsHash(
                    _newName,
                    _newSymbol,
                    _newDecimals
                ) ==
                detailsHash,
            "!committed details"
        );
        // careful with naming convention change here
        token.name = _newName;
        token.symbol = _newSymbol;
        token.decimals = _newDecimals;
        if (!_isFirstDetails) {
            emit UpdateDetails(_newName, _newSymbol, _newDecimals);
        }
    }


    // ============ Public Functions ============

    /**
     * @dev silence the compiler being dumb
     */
    function balanceOf(address _account)
        public
        view
        override(IBridgeWrapperToken, WrapperERC20)
        returns (uint256)
    {
        return WrapperERC20.balanceOf(_account);
    }

    /**
     * @dev Returns the name of the token.
     */
    function name() public view override returns (string memory) {
        return token.name;
    }

    /**
     * @dev Returns the symbol of the token, usually a shorter version of the
     * name.
     */
    function symbol() public view override returns (string memory) {
        return token.symbol;
    }

    /**
     * @dev Returns the number of decimals used to get its user representation.
     * For example, if `decimals` equals `2`, a balance of `505` tokens should
     * be displayed to a user as `5,05` (`505 / 10 ** 2`).
     * Tokens usually opt for a value of 18, imitating the relationship between
     * Ether and Wei. This is the value {ERC20} uses, unless {_setupDecimals} is
     * called.
     * NOTE: This information is only used for _display_ purposes: it in
     * no way affects any of the arithmetic of the contract, including
     * {IERC20-balanceOf} and {IERC20-transfer}.
     */
    function decimals() public view override returns (uint8) {
        return token.decimals;
    }


    // required for solidity inheritance
    function transferOwnership(address _newOwner)
        public
        override(IBridgeWrapperToken, OwnableUpgradeable)
        onlyOwner
    {
        OwnableUpgradeable.transferOwnership(_newOwner);
    }

    /**
     * @dev should be impossible to renounce ownership;
     * we override OpenZeppelin OwnableUpgradeable's
     * implementation of renounceOwnership to make it a no-op
     */
    function renounceOwnership() public override onlyOwner {
        // do nothing
    }
}
