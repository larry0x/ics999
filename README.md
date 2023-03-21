# ICS-999

An all-in-one IBC protocol providing fungible token transfer, interchain account, and query functionalities, implemented in [CosmWasm](https://github.com/CosmWasm/cosmwasm).

## How to Use

Install [cargo-make](https://sagiegurari.github.io/cargo-make/):

```shell
cargo install --force cargo-make
```

Run formatter:

```shell
cargo make fmt
```

Run unit tests:

```shell
cargo make test
```

Run end-to-end tests:

```shell
cargo make e2e
```

Run linter (clippy):

```shell
cargo make lint
```

Check for unused dependencies:

```shell
cargo make udeps
```

Compile all contracts using [rust-optimizer](https://github.com/CosmWasm/rust-optimizer):

```shell
cargo make optimize
```

Once optimized, verify the wasm binaries are ready to be uploaded to the blockchain:

```shell
cargo make check
```

Generate JSON schema for all contracts:

```shell
cargo make schema
```

Publish contracts and packages to [crates.io](https://crates.io/):

```shell
cargo make publish
```

## Copyright

ICS-999 © 2023 [larry0x](https://twitter.com/larry0x)

ICS-999, including its specification and Rust implementation, is a proprietary software owned solely by [larry0x](https://twitter.com/larry0x). All rights reserved.
