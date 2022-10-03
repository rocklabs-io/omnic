// SPDX-License-Identifier: BUSL-1.1

pragma solidity ^0.8.9;
pragma abicoder v2;

// imports external
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/math/SafeMath.sol";

// imports internal
import "./utils/LPTokenERC20.sol";

// one pool manages one token
contract Pool is LPTokenERC20, ReentrancyGuard {
    using SafeMath for uint256;

    //------------------------------- constants --------------------------------------------
    // function selector
    bytes4 private constant SELECTOR =
        bytes4(keccak256(bytes("transfer(address,uint256)")));

    //------------------------------- variables --------------------------------------------
    // metadata
    uint256 public immutable poolId; // shared id between chains to represent same pool
    address public immutable token; // the token for the pool
    uint256 public sharedDecimals; // the shared decimals (lowest common decimals between chains)
    uint256 public localDecimals; // the decimals for the token
    uint256 public immutable convertRate; // the decimals for the token
    address public immutable router; // the token for the pool

    // liquidity and fee
    uint256 public totalLiquidity; // the total amount of tokens added on this side of the chain (fees + deposits - withdrawals)
    uint256 public fee; // the fee for the pool
    uint256 public feeBalance; // fee balance created from mint fee

    // controller
    bool public pause; // flag to stop this pool

    //----------------------------- events ----------------------------------------------

    event Mint(
        address to,
        uint256 amountLP,
        uint256 amountSD,
        uint256 mintFeeAmountSD
    );
    event Burn(address from, uint256 amountLP, uint256 amountSD);
    event RemoveLiquidity(
        address from,
        uint256 amountLP,
        uint256 amountSD,
        address to
    );
    event Swap(
        uint16 dstchainId,
        uint256 dstPoolId,
        address from,
        uint256 amountSD,
        uint256 fee
    );
    event FeesUpdated(uint256 mintFee);

    //----------------------------- modifiers ----------------------------------------------

    modifier onlyRouter() {
        require(
            msg.sender == router,
            "only the router can call this method"
        );
        _;
    }

    modifier onlyNotPause() {
        require(!pause, "Pool paused");
        _;
    }

    constructor(
        uint256 _poolId,
        address _router,
        address _token,
        uint256 _sharedDecimals,
        uint256 _localDecimals,
        string memory _name,
        string memory _symbol
    ) LPTokenERC20(_name, _symbol) {
        require(
            _sharedDecimals <= _localDecimals,
            "common decimals must be little more than token origin decimals"
        );
        require(_token != address(0x0), "_token cannot be 0x0");
        require(_router != address(0x0), "_router cannot be 0x0");
        poolId = _poolId;
        router = _router;
        token = _token;
        sharedDecimals = _sharedDecimals;
        decimals = uint8(_sharedDecimals);
        localDecimals = _localDecimals;
        convertRate = 10**(uint256(localDecimals).sub(sharedDecimals));
    }

    //----------------------------- router called  functions --------------------------------------
    function addLiquidity(address _to, uint256 _amountLD)
        external
        nonReentrant
        onlyRouter
        returns (uint256)
    {
        return _mint(_to, _amountLD, false); // ignore fee now
    }

    function removeLiquidity(
        address _from,
        uint256 _amountLP,
        address _to
    ) external nonReentrant onlyRouter returns (uint256 amountSD) {
        require(_from != address(0x0), "_from cannot be 0x0");
        amountSD = _burnLP(_from, _amountLP);
        uint256 amountLD = amountSDtoLD(amountSD);
        _safeTransfer(token, _to, amountLD);
        emit RemoveLiquidity(_from, _amountLP, amountLD, _to);
    }

    function swap(
        uint16 _dstChainId,
        uint256 _dstPoolId,
        address _from,
        uint256 _amountLD,
        uint256 _minAmountLD
    ) external nonReentrant onlyRouter onlyNotPause {
        uint256 amountSD = amountLDtoSD(_amountLD);
        uint256 minAmountSD = amountLDtoSD(_minAmountLD);
        // update the new amount the user gets minus the fees
        amountSD = amountSD.sub(fee);
        require(amountSD >= minAmountSD, "slippage too high");

        emit Swap(_dstChainId, _dstPoolId, _from, amountSD, fee);
    }

    function setFee(uint256 _mintFee) external onlyRouter {
        fee = _mintFee;
        emit FeesUpdated(fee);
    }

    //----------------------------- utils  functions --------------------------------------

    function amountLPtoLD(uint256 _amountLP) external view returns (uint256) {
        return amountSDtoLD(_amountLPtoSD(_amountLP));
    }

    function _amountLPtoSD(uint256 _amountLP) internal view returns (uint256) {
        require(
            totalSupply > 0,
            "Stargate: cant convert LPtoSD when totalSupply == 0"
        );
        return _amountLP.mul(totalLiquidity).div(totalSupply);
    }

    function _amountSDtoLP(uint256 _amountSD) internal view returns (uint256) {
        require(
            totalLiquidity > 0,
            "Stargate: cant convert SDtoLP when totalLiq == 0"
        );
        return _amountSD.mul(totalSupply).div(totalLiquidity);
    }

    function amountSDtoLD(uint256 _amount) internal view returns (uint256) {
        return _amount.mul(convertRate);
    }

    function amountLDtoSD(uint256 _amount) internal view returns (uint256) {
        return _amount.div(convertRate);
    }

    //----------------------------- internal  functions --------------------------------------
    function _mint(
        address _to,
        uint256 _amountLD,
        bool _feesEnabled
    ) internal returns (uint256 amountSD) {
        amountSD = amountLDtoSD(_amountLD);

        if (_feesEnabled) {
            amountSD = amountSD.sub(fee);
            feeBalance = feeBalance.add(fee);
        }

        uint256 amountLPTokens = amountSD;
        if (totalSupply != 0) {
            amountLPTokens = amountSD.mul(totalSupply).div(totalLiquidity);
        }

        totalLiquidity = totalLiquidity.add(amountSD);

        _mint(_to, amountLPTokens);
        emit Mint(_to, amountLPTokens, amountSD, fee);
    }

    function _burnLP(address _from, uint256 _amountLP)
        internal
        returns (uint256 amountSD)
    {
        require(totalSupply > 0, "no LP token");
        uint256 amountOfLPTokens = balances[_from];
        require(amountOfLPTokens >= _amountLP, "not enough LP tokens to burn");

        amountSD = _amountLP.mul(totalLiquidity).div(totalSupply);
        totalLiquidity = totalLiquidity.sub(amountSD);

        _burn(_from, _amountLP);
        emit Burn(_from, _amountLP, amountSD);
    }

    function _safeTransfer(
        address _token,
        address _to,
        uint256 _value
    ) private {
        (bool success, bytes memory data) = _token.call(
            abi.encodeWithSelector(SELECTOR, _to, _value)
        );
        require(
            success && (data.length == 0 || abi.decode(data, (bool))),
            "Stargate: TRANSFER_FAILED"
        );
    }
}
