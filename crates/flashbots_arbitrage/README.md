### Reserve Tracking: Evolution of Approach

**Initial Implementation (Swap Events):**
I initially tracked reserves by calculating state changes from Swap events:
```rust
reserves.0 += amount0_in - amount0_out;
reserves.1 += amount1_in - amount1_out;
```

This approach taught me:
- Uniswap V2 constant product mechanics (x * y = k)
- How fees impact reserve calculations (γ = 0.997)
- Edge cases: Mint/Burn events require separate handling

**Production Implementation (Sync Events):**
After some research I realised that swap wasn't the only event
that changes pool reserves (mint, burn, multihop).
I refactored to use Sync events:
```rust
// Before (Swap events): Manual calculation
reserves.0 = reserves.0 + amount0_in - amount0_out;
reserves.1 = reserves.1 + amount1_in - amount1_out;

// After (Sync events): Direct state update  
reserves = (sync.reserve0, sync.reserve1);
```

Benefits:
- ✅ Single source of truth (no drift)
- ✅ Captures all state changes (Swap + Mint + Burn)
- ✅ Simpler code (direct state update)
