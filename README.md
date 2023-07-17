# ICS-999

An all-in-one IBC protocol providing fungible token transfer, interchain account (ICA), and query (ICQ) functionalities, implemented in [CosmWasm][cosmwasm].

## Requirements

ICS-999 requires the following in order to work:

- [wasmd][wasmd] >= 0.32
- [tokenfactory][tf] module
- tokenfactory's `denom_creation_fee` must be zero
- tokenfactory's `Params` StargateQuery must be whitelisted ([example][stargate-query])

## Acknowledgement

We thank the authors of the following open source works, which ICS-999 took inspiration from:

- [ICS-20][ics20] and [ICS-27][ics27] specifications, as well as their [Go implementations][ibc-go]
- [Polytone][polytone]

## License

(c) larry0x, 2023 - [All rights reserved](./LICENSE).

[cosmwasm]:       https://github.com/CosmWasm/cosmwasm
[ibc-go]:         https://github.com/cosmos/ibc-go
[ics20]:          https://github.com/cosmos/ibc/tree/main/spec/app/ics-020-fungible-token-transfer
[ics27]:          https://github.com/cosmos/ibc/tree/main/spec/app/ics-027-interchain-accounts
[polytone]:       https://github.com/DA0-DA0/polytone
[stargate-query]: https://github.com/CosmosContracts/juno/blob/v15.0.0/app/keepers/keepers.go#L382-L402
[tf]:             https://github.com/osmosis-labs/osmosis/tree/main/x
[wasmd]:          https://github.com/CosmWasm/wasmd
