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

## License

(c) larry0x, 2023 - [All rights reserved](./LICENSE).
