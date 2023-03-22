# ICS-999

An all-in-one IBC protocol providing fungible token transfer, interchain account (ICA), and query (ICQ) functionalities, implemented in [CosmWasm](https://github.com/CosmWasm/cosmwasm).

## Overview

See 👉 [here](./docs/README.md) for an introduction to the ICS-999 protocol.

## How to Use

Install just: https://github.com/casey/just

Run linter:

```bash
just clippy
```

Run unit tests:

```bash
just test
```

Run end-to-end tests:

```bash
just e2e
```

Compile all contracts using [rust-optimizer](https://github.com/CosmWasm/rust-optimizer):

```bash
just optimize
```

## Copyright

ICS-999 © 2023 [larry0x](https://twitter.com/larry0x)

ICS-999, including its specification and Rust implementation, is a proprietary software owned solely by [larry0x](https://twitter.com/larry0x). All rights reserved.
