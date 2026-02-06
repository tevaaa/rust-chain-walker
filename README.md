# 🦀 Chain Walker

> **A collection of high-performance, low-level EVM tools built in Rust.** 

---

## 🏗  Workspace structure - Roadmap
| Package | Description | Status |
| :--- | :--- | :--- |
| [**rpc_surgeon**](./crates/rpc_surgeon) | Direct storage slot analysis & mapping derivation. | ✅ Stable |
| [**event_horizon**](./crates/event_horizon) | Real-time indexing via asynchronous WebSockets. | ✅ Stable |
| [**v1_legacy_arb**](./crates/flashbots_arbitrage) | Educational MEV engine (Proof-of-Depth). | 📦 Archived |

---
# Rpc Surgeon
Not relying on contract ABIs and `balanceOf()` calls, `rpc_surgeon` calculates the exact memory location of data on the blockchain.
To find the address in a `mapping(address => uint256)` We derive the storage slot using:
$$slot = keccak256(h(k) + p)$$
*Where *k* is the padded address and *p* is the mapping's position.*

- Quick Start:
1. Add your RPC provider to a `.env` file
2. Run example: Get Binance's WETH balance (Slot 3)   
`cargo run -p rpc_surgeon -- -c 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 -o 0xF977814e90dA44bFA03b6295A0616a897441aceC -s 3`

# Event Horizon
**Auto-Configuring**: Automatically fetches token decimals using `eth_call` before starting the subscription.
**Resilient Stream**: Implemented with a reconnection loop and incremental backoff to handle WebSocket drops.
**CLI-First**: Monitor any contract with precision.
**No-Lib**: Manual parsing of `string` / `bytes` from RPC hex responses.

- Quick Start:
1. Add your WSS provider to a `.env` file `WSS_URL = wss://..../KEY` 
2. Run example monitor WETH: `cargo run -p event_horizon -- --target 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2`  

# 🔥 Flashbots Arbitrage (Legacy V1)

This module serves as a **Post-Mortem** of my initial MEV research. It focuses on the "No-Library" approach to understand EVM transactions in depth.

### 🧠 Core Engineering Principles:
- **Zero-Dependency ABI**: Manual construction of calldata to bypass the overhead of standard libraries.
- **Precision Analysis**: Implemented a Ternary Search algorithm for profit maximization.

### 📐 The Math (V2 Constant Product)
Optimal swap amount calculated via:
$$a_{optimal} = \sqrt{\gamma R_1 R_2 \frac{p_2}{p_1}} - R_1$$

### 🚀 Usage (Simulation Mode)
1. **Fork Mainnet**: `anvil --fork-url "HTTPS_RPC_URL"`
2. **Start Monitor**: `cargo run --bin flashbots_arbitrage`
3. **Trigger Arb**: `cargo run --bin market_maker -- --mode extreme`

### ⚠️ Educational Purpose
This is an educational project. Production-grade MEV requires < 10ms end-to-end latency, institutional capital, and private RPC nodes.

