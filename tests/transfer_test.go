package e2e_test

import (
	"encoding/json"

	"github.com/stretchr/testify/require"

	sdk "github.com/cosmos/cosmos-sdk/types"

	wasmibctesting "github.com/CosmWasm/wasmd/x/wasm/ibctesting"
	wasmvmtypes "github.com/CosmWasm/wasmvm/types"

	"ics999/tests/types"
)

var (
	mockRecipient, _         = sdk.AccAddressFromBech32("cosmos1z926ax906k0ycsuckele6x5hh66e2m4mjchwmp")
	mockInitialBalance int64 = 100_000_000
)

// TestMultipleTransfers tests sending multiple coins to multiple recipients in
// a single packet.
func (suite *testSuite) TestMultipleTransfers() {
	// the first two transfers we specify a recipient
	// the other two we don't specify a recipient; should default to the ICA
	_, ack, err := act(suite.chainA, suite.pathAB, []types.Action{
		{
			Transfer: &types.TransferAction{
				Denom:     "uastro",
				Amount:    sdk.NewInt(888_888),
				Recipient: mockRecipient.String(),
			},
		},
		{
			Transfer: &types.TransferAction{
				Denom:     "umars",
				Amount:    sdk.NewInt(69_420),
				Recipient: mockRecipient.String(),
			},
		},
		{
			RegisterAccount: &types.RegisterAccountAction{},
		},
		{
			Transfer: &types.TransferAction{
				Denom:  "uastro",
				Amount: sdk.NewInt(987_654),
			},
		},
		{
			Transfer: &types.TransferAction{
				Denom:  "umars",
				Amount: sdk.NewInt(1_111_111),
			},
		},
	})
	require.NoError(suite.T(), err)
	requirePacketSuccess(suite.T(), ack)

	// predict what the denom would be
	astroVoucherDenom := deriveVoucherDenom(suite.chainB, []*wasmibctesting.Path{suite.pathAB}, "uastro")
	marsVoucherDenom := deriveVoucherDenom(suite.chainB, []*wasmibctesting.Path{suite.pathAB}, "umars")

	// recipient unspecified, default to the ICA
	icaAddr, err := sdk.AccAddressFromBech32(ack.Results[2].RegisterAccount.Address)
	require.NoError(suite.T(), err)

	// sender balance on chainA should have been reduced
	// recipient balance on chainB should have been increased
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.senderAddr, "uastro", mockInitialBalance-888_888-987_654)
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.senderAddr, "umars", mockInitialBalance-69_420-1_111_111)
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.coreAddr, "uastro", 888_888+987_654)
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.coreAddr, "umars", 69_420+1_111_111)
	requireBalanceEqual(suite.T(), suite.chainB, mockRecipient, astroVoucherDenom, 888_888)
	requireBalanceEqual(suite.T(), suite.chainB, mockRecipient, marsVoucherDenom, 69_420)
	requireBalanceEqual(suite.T(), suite.chainB, icaAddr, astroVoucherDenom, 987_654)
	requireBalanceEqual(suite.T(), suite.chainB, icaAddr, marsVoucherDenom, 1_111_111)
}

// TestSequentialTransfers in this test, to do the following transfer:
//
//	chainA --> chainB --> chainC --> chainB
//
// The objective is to test in the last step, whether the voucher tokens are
// properly burned and escrowed tokens released.
func (suite *testSuite) TestSequentialTransfers() {
	var (
		// astro token's denom on chainB
		astroB = deriveVoucherDenom(suite.chainB, []*wasmibctesting.Path{suite.pathAB}, "uastro")
		// astro token's denom on chainC
		astroC = deriveVoucherDenom(suite.chainC, []*wasmibctesting.Path{suite.pathAB, suite.pathBC}, "uastro")

		// how many astro to send chainA --> chainB
		amountAB int64 = 12345
		// how many astro to send chainB --> chainC
		amountBC int64 = 10000
		// how many astro to send chainC --> chainB
		amountCB int64 = 8964
	)

	// chainA --> chainB
	_, ack, err := act(suite.chainA, suite.pathAB, []types.Action{
		{
			Transfer: &types.TransferAction{
				Denom:     "uastro",
				Amount:    sdk.NewInt(amountAB),
				Recipient: suite.chainB.senderAddr.String(),
			},
		},
	})
	require.NoError(suite.T(), err)
	requirePacketSuccess(suite.T(), ack)

	// chainB --> chainC
	_, ack, err = act(suite.chainB, suite.pathBC, []types.Action{
		{
			Transfer: &types.TransferAction{
				Denom:     astroB,
				Amount:    sdk.NewInt(amountBC),
				Recipient: suite.chainC.senderAddr.String(),
			},
		},
	})
	require.NoError(suite.T(), err)
	requirePacketSuccess(suite.T(), ack)

	// astro of amountAB should have been escrowed on chainA
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.senderAddr, "uastro", mockInitialBalance-amountAB)
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.coreAddr, "uastro", amountAB)
	// astroB of amountAB should have been minted on chainB
	// astroB of amountBC should have been escrowed on chainB
	requireBalanceEqual(suite.T(), suite.chainB, suite.chainB.senderAddr, astroB, amountAB-amountBC)
	requireBalanceEqual(suite.T(), suite.chainB, suite.chainB.coreAddr, astroB, amountBC)
	// astroC of amountBC should have been minted on chainC
	requireBalanceEqual(suite.T(), suite.chainC, suite.chainC.senderAddr, astroC, amountBC)
	requireBalanceEqual(suite.T(), suite.chainC, suite.chainC.coreAddr, astroC, 0)

	// verify denom traces
	requireTraceEqual(suite.T(), suite.chainB, astroB, types.Trace{
		Denom:     astroB,
		BaseDenom: "uastro",
		Path: []wasmvmtypes.IBCEndpoint{
			{
				PortID:    suite.pathAB.EndpointB.ChannelConfig.PortID,
				ChannelID: suite.pathAB.EndpointB.ChannelID,
			},
		},
	})
	requireTraceEqual(suite.T(), suite.chainC, astroC, types.Trace{
		Denom:     astroC,
		BaseDenom: "uastro",
		Path: []wasmvmtypes.IBCEndpoint{
			{
				PortID:    suite.pathAB.EndpointB.ChannelConfig.PortID,
				ChannelID: suite.pathAB.EndpointB.ChannelID,
			},
			{
				PortID:    suite.pathBC.EndpointB.ChannelConfig.PortID,
				ChannelID: suite.pathBC.EndpointB.ChannelID,
			},
		},
	})

	// chainC --> chainB
	_, ack, err = act(suite.chainC, reversePath(suite.pathBC), []types.Action{
		{
			Transfer: &types.TransferAction{
				Denom:     astroC,
				Amount:    sdk.NewInt(amountCB),
				Recipient: mockRecipient.String(),
			},
		},
	})
	require.NoError(suite.T(), err)
	requirePacketSuccess(suite.T(), ack)

	// astroC of amountCB should have been burned on chainC
	requireBalanceEqual(suite.T(), suite.chainC, suite.chainC.senderAddr, astroC, amountBC-amountCB)
	requireBalanceEqual(suite.T(), suite.chainC, suite.chainC.coreAddr, astroC, 0)
	// astroB of amountCB should have been released from escrow
	requireBalanceEqual(suite.T(), suite.chainB, mockRecipient, astroB, amountCB)
	requireBalanceEqual(suite.T(), suite.chainB, suite.chainB.coreAddr, astroB, amountBC-amountCB)
}

// TestRefund tests the funds escrowed on the sender chain is properly refunded
// if the packet fails to execute.
func (suite *testSuite) TestRefund() {
	// attempt to transfer tokens without specifying a recipient while not having
	// an ICA already registered. should fail
	_, ack, err := act(suite.chainA, suite.pathAB, []types.Action{
		{
			Transfer: &types.TransferAction{
				Denom:  "uastro",
				Amount: sdk.NewInt(12345),
			},
		},
	})
	require.NoError(suite.T(), err)
	requirePacketFailed(suite.T(), ack)

	// escrowed tokens should have been refuneded. user and core contracts' token
	// balances should have been the same as if the escrow never happened
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.senderAddr, "uastro", mockInitialBalance)
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.coreAddr, "uastro", 0)
	requireBalanceEqual(suite.T(), suite.chainB, suite.chainB.senderAddr, "uastro", 0)
	requireBalanceEqual(suite.T(), suite.chainB, suite.chainB.coreAddr, "uastro", 0)
}

// TestSwap the most complex test - we send coins from chainA to chainB, make a
// swap at a DEX contract on chainB, then send the proceedings back to chainA,
// all in the same packet.
func (suite *testSuite) TestSwap() {
	var (
		// astro token's denom on chainB
		astroB = deriveVoucherDenom(suite.chainB, []*wasmibctesting.Path{suite.pathAB}, "uastro")

		// usdc token's denom on chainA
		usdcA = deriveVoucherDenom(suite.chainA, []*wasmibctesting.Path{reversePath(suite.pathAB)}, "uusdc")

		// how many astro to send chainA --> chainB and be swapped
		amountAB int64 = 12345

		// how many USDC the seed the DEX
		dexInitialBalance int64 = 23456
	)

	// deploy mock-dex contract on chainB
	dexStoreRes := suite.chainB.StoreCodeFile("../artifacts/mock_dex.wasm")
	dexInstantiateMsg, err := json.Marshal(&types.DexInstantiateMsg{
		DenomIn:  astroB,
		DenomOut: "uusdc",
	})
	require.NoError(suite.T(), err)
	dexAddr := suite.chainB.InstantiateContract(dexStoreRes.CodeID, dexInstantiateMsg)

	// fund the dex with USDC
	mintCoinsToAccount(suite.chainB.TestChain, dexAddr, sdk.NewCoin("uusdc", sdk.NewInt(dexInitialBalance)))

	// execute the actions:
	// - register an interchain account
	// - send ASTRO to the ICA
	// - swap ASTRO for USDC
	// - send USDC back
	swapMsg, err := json.Marshal(&types.DexExecuteMsg{
		Swap: &types.DexSwap{},
	})
	require.NoError(suite.T(), err)

	sendBackMsg, err := json.Marshal(&types.CoreExecuteMsg{
		Act: &types.Act{
			ConnectionID: suite.pathAB.EndpointB.ConnectionID,
			Actions: []types.Action{
				{
					Transfer: &types.TransferAction{
						Denom:     "uusdc",
						Amount:    sdk.NewInt(amountAB),
						Recipient: suite.chainA.senderAddr.String(),
					},
				},
			},
		},
	})
	require.NoError(suite.T(), err)

	_, ack, err := act(suite.chainA, suite.pathAB, []types.Action{
		{
			RegisterAccount: &types.RegisterAccountAction{},
		},
		{
			Transfer: &types.TransferAction{
				Denom:  "uastro",
				Amount: sdk.NewInt(amountAB),
			},
		},
		{
			Execute: &wasmvmtypes.CosmosMsg{
				Wasm: &wasmvmtypes.WasmMsg{
					Execute: &wasmvmtypes.ExecuteMsg{
						ContractAddr: dexAddr.String(),
						Msg:          swapMsg,
						Funds:        []wasmvmtypes.Coin{wasmvmtypes.NewCoin(uint64(amountAB), astroB)},
					},
				},
			},
		},
		{
			Execute: &wasmvmtypes.CosmosMsg{
				Wasm: &wasmvmtypes.WasmMsg{
					Execute: &wasmvmtypes.ExecuteMsg{
						ContractAddr: suite.chainB.coreAddr.String(),
						Msg:          sendBackMsg,
						Funds:        []wasmvmtypes.Coin{wasmvmtypes.NewCoin(uint64(amountAB), "uusdc")},
					},
				},
			},
		},
	})
	require.NoError(suite.T(), err)
	requirePacketSuccess(suite.T(), ack)

	// relay the packet that send the USDC back to chainA
	_, ackBytes, err := relaySinglePacket(reversePath(suite.pathAB))
	require.NoError(suite.T(), err)

	ack = &types.PacketAck{}
	err = json.Unmarshal(ackBytes, ack)
	require.NoError(suite.T(), err)

	requirePacketSuccess(suite.T(), ack)

	// verify balances are correct
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.senderAddr, "uastro", mockInitialBalance-amountAB)
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.senderAddr, usdcA, amountAB)
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.coreAddr, "uastro", amountAB)
	requireBalanceEqual(suite.T(), suite.chainB, suite.chainA.coreAddr, "uusdc", amountAB)
	requireBalanceEqual(suite.T(), suite.chainB, dexAddr, astroB, amountAB)
	requireBalanceEqual(suite.T(), suite.chainB, dexAddr, "uusdc", dexInitialBalance-amountAB)
}
