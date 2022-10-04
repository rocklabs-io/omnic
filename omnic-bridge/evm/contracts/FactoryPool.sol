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

    mapping(uint256 => Pool) public pools; // poolId -> PoolInfo
    address[] public allPools;
    address public immutable router;

    //---------------------------------------------------------------------------
    // MODIFIERS
    modifier onlyRouter() {
        require(msg.sender == router, "Stargate: caller must be Router.");
        _;
    }

    constructor(address _router) {
        require(_router != address(0x0), "Stargate: _router cant be 0x0"); // 1 time only
        router = _router;
    }

    function allPoolsLength() external view returns (uint256) {
        return allPools.length;
    }

    function createPool(
        uint256 _poolId,
        address _token,
        uint8 _sharedDecimals,
        uint8 _localDecimals,
        string memory _name,
        string memory _symbol
    ) public onlyRouter returns (address poolAddress) {
        require(
            address(pools[_poolId]) == address(0x0),
            "Pool already created"
        );

        Pool pool = new Pool(
            _poolId,
            router,
            _token,
            _sharedDecimals,
            _localDecimals,
            _name,
            _symbol
        );
        pools[_poolId] = pool;
        poolAddress = address(pool);
        allPools.push(poolAddress);
    }

    function renounceOwnership() public override onlyOwner {}
}
