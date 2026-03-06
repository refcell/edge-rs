# Edge Standard Library

The Edge standard library (`std/`) provides reusable contracts, traits, and utility functions for building EVM smart contracts in the Edge language.

## Directory Structure

```
std/
├── math.edge                    # WAD/RAY fixed-point and safe arithmetic
├── auth.edge                    # Ownership (Owned) and authority (Auth) primitives
├── math/
│   └── signed_wad.edge          # Signed fixed-point WAD arithmetic
├── tokens/
│   ├── erc20.edge               # ERC-20 fungible token
│   ├── erc721.edge              # ERC-721 non-fungible token
│   ├── erc1155.edge             # ERC-1155 multi-token standard
│   ├── erc4626.edge             # ERC-4626 tokenized vault
│   ├── weth.edge                # Wrapped ETH (WETH)
│   └── safe_transfer.edge       # Safe ERC-20 and ETH transfer utilities
├── access/
│   ├── ownable.edge             # 2-step ownership transfer pattern
│   ├── pausable.edge            # Pausable contract pattern
│   └── roles.edge               # Role-based access control (RBAC)
├── utils/
│   ├── merkle.edge              # Merkle proof verification
│   ├── bits.edge                # Bit manipulation utilities
│   ├── bytes.edge               # Bytes32 conversion utilities
│   └── create2.edge             # CREATE2 address computation
├── patterns/
│   ├── reentrancy_guard.edge    # Reentrancy protection (persistent + transient)
│   ├── timelock.edge            # Time-locked operations
│   └── factory.edge             # CREATE2 deterministic deployment factory
└── finance/
    ├── amm.edge                 # Constant product AMM (x * y = k)
    ├── staking.edge             # ERC-20 staking with per-second rewards
    └── multisig.edge            # N-of-M multisig wallet
```

## Usage

Import standard library modules using the `std::` prefix:

```edge
// Import a specific interface
use std::tokens::erc20::IERC20;

// Import a utility module
use std::math;
let result: u256 = math::wad_mul(a, b);

// Import access control
use std::auth::IOwned;
use std::access::ownable::IOwnable;

// Import patterns
use std::patterns::reentrancy_guard;
```

## Modules

### Core

| Module | Description |
|--------|-------------|
| `std::math` | WAD (1e18) and RAY (1e27) fixed-point arithmetic, safe add/sub, min/max/clamp, mul_div |
| `std::auth` | Owned (single-owner) and Auth (owner + authority) access control primitives |
| `std::math::signed_wad` | Signed fixed-point WAD multiplication and division |

### Tokens

| Module | Description |
|--------|-------------|
| `std::tokens::erc20` | Full ERC-20 with balances, allowances, mint, burn, infinite approval pattern |
| `std::tokens::erc721` | ERC-721 NFT with approve, transferFrom, safeTransferFrom |
| `std::tokens::erc1155` | ERC-1155 multi-token with per-ID balances and operator approvals |
| `std::tokens::erc4626` | ERC-4626 tokenized vault with deposit, withdraw, redeem, share conversion |
| `std::tokens::weth` | Wrapped ETH with deposit/withdraw and full ERC-20 interface |
| `std::tokens::safe_transfer` | Safe ERC-20 transfer wrappers and ETH transfer utilities |

### Access Control

| Module | Description |
|--------|-------------|
| `std::access::ownable` | 2-step ownership transfer with pending owner pattern |
| `std::access::pausable` | Pausable pattern with owner-gated pause/unpause |
| `std::access::roles` | Role-based access control with admin hierarchy |

### Utilities

| Module | Description |
|--------|-------------|
| `std::utils::merkle` | Merkle proof verification against a root hash |
| `std::utils::bits` | Bit manipulation: MSB, LSB, popcount, power-of-two check |
| `std::utils::bytes` | Bytes32 utilities: address extraction, packing, slicing |
| `std::utils::create2` | CREATE2 address computation and deployment helpers |

### Patterns

| Module | Description |
|--------|-------------|
| `std::patterns::reentrancy_guard` | Reentrancy protection using persistent or transient storage |
| `std::patterns::timelock` | Time-locked operation scheduling and execution |
| `std::patterns::factory` | CREATE2 deterministic deployment factory |

### Finance

| Module | Description |
|--------|-------------|
| `std::finance::amm` | Constant product AMM with LP token minting |
| `std::finance::staking` | ERC-20 staking pool with per-second reward distribution |
| `std::finance::multisig` | N-of-M multisig wallet with proposal lifecycle |
