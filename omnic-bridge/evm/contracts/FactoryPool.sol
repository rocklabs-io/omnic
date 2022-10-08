// SPDX-License-Identifier: BUSL-1.1

pragma solidity ^0.8.9;
pragma abicoder v2;

//imports external
import "@openzeppelin/contracts/utils/math/SafeMath.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

// imports internal
import "./Pool.sol";

contract FactoryPool is Ownable {
    using SafeMath for uint256;

    //---------------------------- variables -----------------------------------------------

    mapping(address => uint256) public getPoolId; // token_addr -> poolId
    mapping(uint256 => Pool) public pools; // poolId -> PoolInfo
    address[] public allPools;
    address public immutable router;

    //---------------------------------------------------------------------------
    // MODIFIERS
    modifier onlyRouter() {
        require(msg.sender == router, "caller must be Router.");
        _;
    }

    constructor(address _router) {
        require(_router != address(0x0), "_router cant be 0x0"); // 1 time only
        router = _router;
    }

    function allPoolsLength() external view returns (uint256) {
        return allPools.length;
    }

    function createPool(
        address _token,
        uint8 _sharedDecimals,
        uint8 _localDecimals,
        string memory _name,
        string memory _symbol
    ) public onlyRouter returns (address, uint256) {
        uint256 poolId = allPools.length;
        // TODO: check if pool for this token already exist
        Pool pool = new Pool(
            poolId,
            router,
            _token,
            _sharedDecimals,
            _localDecimals,
            _name,
            _symbol
        );
        getPoolId[_token] = poolId;
        pools[poolId] = pool;
        address poolAddress = address(pool);
        allPools.push(poolAddress);
        return (poolAddress, poolId);
    }

    function renounceOwnership() public override onlyOwner {}
}
