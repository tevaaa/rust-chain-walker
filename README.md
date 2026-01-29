# 🦀 Chain Walker

> **A collection of high-performance, low-level EVM tools built in Rust.** 

---

## 🏗  Workspace structure - Roadmap
| Package | Description | Status |
| :--- | :--- | :--- |
| [**rpc_surgeon**](./crates/rpc_surgeon) | Direct storage slot analysis & mapping derivation. | ✅ Stable |
| [**event_horizon**](./crates/event_horizon) | Real-time indexing via asynchronous WebSockets. | ✅ Stable |
| [**flashbots_sniper**](./crates/flashbots_sniper) | Bypass the public mempool | 🛠  In progress |
| [**evm_disassembler**](./crates/flashbots_sniper) | Dissect bytecode into human-readable Assenbly | ⏳ Planned |


---
## Rpc Surgeon
Not relying on contract ABIs and `balanceOf()` calls, `rpc_surgeon` calculates the exact memory location of data on the blockchain.
To find the address in a `mapping(address => uint256)` We derive the storage slot using:
$$slot = keccak256(h(k) + p)$$
*Where *k* is the padded address and *p* is the mapping's position.*

- Quick Start:
1. Add your RPC provider to a `.env` file
2. Run example: Get Binance's WETH balance (Slot 3)   
`cargo run -p rpc_surgeon -- -c 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 -o 0xF977814e90dA44bFA03b6295A0616a897441aceC -s 3`

## Event Horizon
**Auto-Configuring**: Automatically fetches token decimals using `eth_call` before starting the subscription.
**Resilient Stream**: Implemented with a reconnection loop and incremental backoff to handle WebSocket drops.
**CLI-First**: Monitor any contract with precision.
**No-Lib**: Manual parsing of `string` / `bytes` from RPC hex responses.

- Quick Start:
1. Add your WSS provider to a `.env` file `WSS_URL = wss://..../KEY` 
2. Run example monitor WETH: `cargo run -p event_horizon -- --target 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2`  

