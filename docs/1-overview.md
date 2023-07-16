# Overview

ICS-999 is an IBC application layer protocol with fungible token transfer, interchain account, and interchain query capabilities.

That's a lot of jargons! Let's break them down.

## IBC architecture

[IBC][ibc] (**I**nter-**B**lockchain **C**ommunication) is a general-purpose cross-chain messaging protocol developed in the [Cosmos ecosystem][cosmos].

IBC's architecture consists of two layers:

- **transport layer**

  Handles the transportation of data packets between two blockchains.

  After a packet is emitted by the sender chain, an actor known as the **relayer** will post the packet on the receiver chain, along with a Merkle proof that proves the packet is authentic. The receiver chain will verify the proof, and then hands the packet over to the application layer to be processed.

  A key characteristic of the transport layer is that the data packets are in the form of raw binary (0s and 1s). The transport layer does not attempt to interpret the meaning of those bits; that is the job of the app layer.

- **application layer**

  On receiving a verified packet from the transport layer, the app layer interprets the packet and processes it.

  There are many app layer protocols, each doing a different job. They can be implemented either as [Cosmos SDK][cosmos-sdk] modules or [CosmWasm][cosmwasm] contracts. The app layer identifies which protocol the packet is meant for and dispatches the packet to it.

There are a lotta more technical details we need to know in order to fully understand how IBC works, such as light clients, connections, ports, channels, handshake, acknowledgement, timeout... We'll cover them in the following chapters of this guide series.

ICS-999 specifically, is an app layer protocol.

## IBC app layer protocols

There are 3 major IBC app layer protocols currently in use today, each with an ICS (**I**nter**c**hain **S**tandard) identifier:

| protocol        | what is does                                                              |
| --------------- | ------------------------------------------------------------------------- |
| [ICS-20][ics20] | send tokens across chains                                                 |
| [ICS-27][ics27] | execute state-mutating actions on another chain, aka "interchain account" |
| [ICS-31][ics31] | exeucte state non-mutating queries on another chain                       |

ICS-999 is an _all-in-one protocol_ that has the capabilities of all of them, and therefore competes with them at the same time.

Now, the obvious question is why building ICS-999 when there are already working protocols that do the same things? The answer is that those protocols are... simply not very good. I explained specifically why they aren't good in my [AwesomWasm 2023][awesomwasm] tech talk (**VIDEO LINK TBD**), but the chart below gives a brief summary of 8 things that ICS-20/27/31 do poorly but ICS-999 does well:

| ICS-20/27/31                                            | ICS-999                                              |
| ------------------------------------------------------- | ---------------------------------------------------- |
| ❌ multiple packets needed to perform complex actions    | ✅ a single packet                                    |
| ❌ impossible to enforce order between channels          | ✅ actions within the single packet are ordered       |
| ❌ not atomic                                            | ✅ atomic                                             |
| ❌ does not provides callback                            | ✅ provides callback                                  |
| ❌ only 1 token per packet                               | ✅ send as many tokens as you want in a single packet |
| ❌ multiple channels between the same two chains allowed | ✅ only 1 channel allowed, no user confusion          |
| ❌ no CW bindings, protobuf difficult to work with       | ✅ built entirely in CW, packet data in json          |
| ❌ ordered channel, easily drop dead                     | ✅ unordered channel that can never be closed         |

## The mandate of ICS-999

**ICS-999 aims to be the _only_ IBC app layer protocol you will ever need to develop interchain smart contract systems, with superior developer experience than the alternatives.**

Let's continue in the following chapters to see how ICS-999 works, and how to integrate it into your project.

[awesomwasm]: https://www.awesomwasm.com/
[cosmos]:     https://cosmos.network/
[cosmos-sdk]: https://github.com/cosmos/cosmos-sdk
[cosmwasm]:   https://cosmwasm.com/
[ibc]:        https://ibcprotocol.org/
[ics20]:      https://github.com/cosmos/ibc/tree/main/spec/app/ics-020-fungible-token-transfer
[ics27]:      https://github.com/cosmos/ibc/tree/main/spec/app/ics-027-interchain-accounts
[ics31]:      https://github.com/cosmos/ibc/tree/main/spec/app/ics-031-crosschain-queries
