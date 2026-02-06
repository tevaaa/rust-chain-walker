// SPDX-License-Identifier: MIT
pragma solidity ^0.8.33;

interface IUniswapV2Pair {
    function swap(uint amount0Out, uint amount1Out, address to, bytes calldata data) external;
}

interface IERC20 {
    function transfer(address to, uint value) external returns (bool);
    function approve(address spender, uint value) external returns (bool);
    function balanceOf(address) external view returns (uint);
}

interface IFlashLoanSimpleReceiver {
    function executeOperation(
        address asset,
        uint256 amount,
        uint256 premium,
        address initiator,
        bytes calldata params
    ) external returns (bool);
}

interface IPool {
    function flashLoanSimple(
        address receiverAddress,
        address asset,
        uint256 amount,
        bytes calldata params,
        uint16 referralCode
    ) external;
}

contract FlashArb is IFlashLoanSimpleReceiver {
    address public immutable owner;
    
    // Aave V3 Pool
    IPool public constant POOL = IPool(0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2);
    
    constructor() {
        owner = msg.sender;
    }
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }
    
    struct ArbParams {
        address tokenIn;      // USDC
        address tokenOut;     // WETH
        address poolBuy;      // Cheap pool
        address poolSell;     // Expensive pool
        uint256 amountIn;     // Amount to borrow
        uint256 amount0OutBuy;
        uint256 amount1OutBuy;
        uint256 amount0OutSell;
        uint256 amount1OutSell;
    }
    
    /// @notice Initiate flash loan arbitrage
    function executeArb(
        address tokenIn,
        address tokenOut,
        address poolBuy,
        address poolSell,
        uint256 amountIn,
        uint256 amount0OutBuy,
        uint256 amount1OutBuy,
        uint256 amount0OutSell,
        uint256 amount1OutSell
    ) external onlyOwner {
        // Encode parameters
        bytes memory params = abi.encode(
            ArbParams({
                tokenIn: tokenIn,
                tokenOut: tokenOut,
                poolBuy: poolBuy,
                poolSell: poolSell,
                amountIn: amountIn,
                amount0OutBuy: amount0OutBuy,
                amount1OutBuy: amount1OutBuy,
                amount0OutSell: amount0OutSell,
                amount1OutSell: amount1OutSell
            })
        );
        
        // Request flash loan from Aave
        POOL.flashLoanSimple(
            address(this),
            tokenIn,        // Asset to borrow (USDC)
            amountIn,       // Amount to borrow
            params,
            0
        );
    }
    
    /// @notice Aave calls this with borrowed funds
    function executeOperation(
        address asset,      // USDC
        uint256 amount,
        uint256 premium,    // Fee (0.05% of amount)
        address initiator,
        bytes calldata params
    ) external returns (bool) {

        require(msg.sender == address(POOL), "Not Aave");
        require(initiator == address(this), "Not initiated by us");
        
        ArbParams memory arb = abi.decode(params, (ArbParams));
        
        uint256 balBefore = IERC20(arb.tokenIn).balanceOf(address(this));
        
        // Send tokenIn to buy pool
        require(IERC20(arb.tokenIn).transfer(arb.poolBuy, arb.amountIn), "Transfer 1 failed");
        
        // Swap on buy pool (receive tokenOut)
        IUniswapV2Pair(arb.poolBuy).swap(
            arb.amount0OutBuy,
            arb.amount1OutBuy,
            address(this),
            ""
        );
        
        // Send tokenOut to sell pool
        uint256 tokenOutBalance = IERC20(arb.tokenOut).balanceOf(address(this));
        require(IERC20(arb.tokenOut).transfer(arb.poolSell, tokenOutBalance), "Transfer 2 failed");
        
        // Swap on sell pool (receive tokenIn back)
        IUniswapV2Pair(arb.poolSell).swap(
            arb.amount0OutSell,
            arb.amount1OutSell,
            address(this),
            ""
        );
    }
}
