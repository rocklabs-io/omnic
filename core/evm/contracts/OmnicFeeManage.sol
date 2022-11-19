// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.9;

// external
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/math/SafeMath.sol";

// internal
import "./interfaces/IOmnicFeeManage.sol";
import "./interfaces/IOmnicNode.sol";

contract OmnicFeeManage is IOmnicFeeManage, Ownable {
    using SafeMath for uint256;

    uint256 public nativeFeeBase;
    uint256 public nativeFeeForPerByte;
    uint256 public erc20FeeBase;
    uint256 public erc20FeeBaseForPerByte;
    bool public feeEnabled;
    bool public erc20Enabled;

    IOmnicNode public omnicNode;

    event NativeFeeBase(uint256 nativeFeeBase);
    event NativeFeeForPerByte(uint256 nativeFeeForPerByte);
    event ERC20FeeBase(uint256 erc20FeeBase);
    event ERC20FeeBaseForPerByte(uint256 erc20FeeBaseForPerByte);
    event FeeEnabled(bool feeEnabled);
    event ERC20Enabled(bool erc20Enabled);

    constructor(address _omnicNode) {
        require(_omnicNode != address(0x0), "OmnicFeeManage: no omnic node");
        omnicNode = IOmnicNode(_omnicNode);
    }

    function getFees(bool payInERC20, uint256 msgLength) external view override returns (uint256) {
        if (feeEnabled) {
            if (payInERC20) {
                require(erc20Enabled, "OmnicFeeManage: ERC20 is not enabled");
                return erc20FeeBase.add(erc20FeeBaseForPerByte.mul(msgLength));
            } else {
                return nativeFeeBase.add(nativeFeeForPerByte.mul(msgLength));
            }
        }
        return 0;
    }

    function setFeeEnabled(bool _feeEnabled) external onlyOwner {
        feeEnabled = _feeEnabled;
        emit FeeEnabled(feeEnabled);
    }

    function setERC20Enabled(bool _erc20Enabled) external onlyOwner {
        erc20Enabled = _erc20Enabled;
        emit ERC20Enabled(erc20Enabled);
    }

    function setNativeFeeBase(uint256 _nativeFeeBase) external onlyOwner {
        nativeFeeBase = _nativeFeeBase;
        emit NativeFeeBase(nativeFeeBase);
    }

    function setERC20FeeBase(uint256 _erc20FeeBase) external onlyOwner {
        erc20FeeBase = _erc20FeeBase;
        emit ERC20FeeBase(erc20FeeBase);
    }

    function setNativeFeeForPerByte(uint256 _nativeFeeForPerByte) external onlyOwner {
        nativeFeeForPerByte = _nativeFeeForPerByte;
        emit NativeFeeBase(nativeFeeForPerByte);
    }

    function setERC20FeeForPerByte(uint256 _erc20FeeBaseForPerByte) external onlyOwner {
        erc20FeeBaseForPerByte = _erc20FeeBaseForPerByte;
        emit ERC20FeeBase(erc20FeeBaseForPerByte);
    }

    function withdrawERC20FeeFromNode(address _to, address _ERC20FeeToken, uint256 _amount) external onlyOwner {
        omnicNode.withdrawERC20Fee(_to, _ERC20FeeToken, _amount);
    }

    function withdrawNativeFeeFromNode(address payable _to, uint256 _amount) external onlyOwner {
        omnicNode.withdrawNativeFee(_to, _amount);
    }
}
