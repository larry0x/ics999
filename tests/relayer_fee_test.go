package e2e_test

import (
	"github.com/stretchr/testify/require"

	sdk "github.com/cosmos/cosmos-sdk/types"

	wasmibctesting "github.com/CosmWasm/wasmd/x/wasm/ibctesting"
	wasmvmtypes "github.com/CosmWasm/wasmvm/types"

	"ics999/tests/types"
)

func (suite *testSuite) TestRelayerFee() {
	marsVoucherDenom := deriveVoucherDenom(suite.chainB, []*wasmibctesting.Path{suite.pathAB}, "umars")

	_, ack, err := actWithRelayerFee(
		suite.chainA,
		suite.pathAB,
		[]types.Action{
			{
				RegisterAccount: &types.RegisterAccountAction{},
			},
		},
		types.RelayerFee{
			Dest: &wasmvmtypes.Coin{Denom: "umars", Amount: "12345"},
			Src:  &wasmvmtypes.Coin{Denom: "uastro", Amount: "23456"},
		},
	)
	require.NoError(suite.T(), err)
	requirePacketSuccess(suite.T(), ack)

	// 12345umars should have been escrowed on the source chain
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.coreAddr, "umars", 12345)
	// relayer on chainB should have received the dest fee
	requireBalanceEqual(suite.T(), suite.chainB, suite.chainB.SenderAccount.GetAddress(), marsVoucherDenom, 12345)
	// relayer on chainA should have received the src fee
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.SenderAccount.GetAddress(), "uastro", 23456)
}

// in the previous test we tested the case where the fee is paid with the sender
// chain's native token (umars). the dest relayer (the one who posts the MsgRecvPacket)
// is paid the voucher token of umars. here we test what if the user pays with
// a voucher token on the source chain. the relayer should receive the unwrapped
// token on the dest chain.
func (suite *testSuite) TestRelayerFeeWithVoucher() {
	usdcVoucherDenom := deriveVoucherDenom(suite.chainA, []*wasmibctesting.Path{reversePath(suite.pathAB)}, "uusdc")

	// first let's send some USDC (the native token of chainB) to chainA
	_, ack, err := act(suite.chainB, reversePath(suite.pathAB), []types.Action{
		{
			Transfer: &types.TransferAction{
				Denom:     "uusdc",
				Amount:    sdk.NewInt(12345),
				Recipient: suite.chainA.senderAddr.String(),
			},
		},
	})
	require.NoError(suite.T(), err)
	requirePacketSuccess(suite.T(), ack)

	// usdc should have been escrowed on chainB
	requireBalanceEqual(suite.T(), suite.chainB, suite.chainB.senderAddr, "uusdc", mockInitialBalance-12345)
	requireBalanceEqual(suite.T(), suite.chainB, suite.chainB.coreAddr, "uusdc", 12345)
	// usdc voucher tokens should have been minted on chainA
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.senderAddr, usdcVoucherDenom, 12345)

	// now, we send a packet chainA->B using the voucher token as fee
	_, ack, err = actWithRelayerFee(
		suite.chainA,
		suite.pathAB,
		[]types.Action{
			{
				RegisterAccount: &types.RegisterAccountAction{},
			},
		},
		types.RelayerFee{
			Dest: &wasmvmtypes.Coin{Denom: usdcVoucherDenom, Amount: "10000"},
		},
	)
	require.NoError(suite.T(), err)
	requirePacketSuccess(suite.T(), ack)

	// usdc voucher tokens should have been burned on chainA
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.senderAddr, usdcVoucherDenom, 12345-10000)
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.coreAddr, usdcVoucherDenom, 0)
	// usdc on chainB should have been released from escrow and sent to the relayer
	requireBalanceEqual(suite.T(), suite.chainB, suite.chainB.coreAddr, "uusdc", 12345-10000)
	requireBalanceEqual(suite.T(), suite.chainB, suite.chainB.SenderAccount.GetAddress(), "uusdc", 10000)
}

// if the packet fails to execute - the destination relayer should get the fee
// (they did the work to relay the packet afterall so they're entitled to it).
// the src relayer should also get it. in other words it should work exactly the
// same as if the packet was successful.
func (suite *testSuite) TestRelayerFeePacketFailed() {
	marsVoucherDenom := deriveVoucherDenom(suite.chainB, []*wasmibctesting.Path{suite.pathAB}, "umars")

	// we attempt to register the interchain account twice which should fail
	_, ack, err := actWithRelayerFee(
		suite.chainA,
		suite.pathAB,
		[]types.Action{
			{
				RegisterAccount: &types.RegisterAccountAction{},
			},
			{
				RegisterAccount: &types.RegisterAccountAction{},
			},
		},
		types.RelayerFee{
			Dest: &wasmvmtypes.Coin{Denom: "umars", Amount: "12345"},
			Src:  &wasmvmtypes.Coin{Denom: "uastro", Amount: "23456"},
		},
	)
	require.NoError(suite.T(), err)
	requirePacketFailed(suite.T(), ack)

	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.coreAddr, "umars", 12345)
	requireBalanceEqual(suite.T(), suite.chainB, suite.chainB.SenderAccount.GetAddress(), marsVoucherDenom, 12345)
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.SenderAccount.GetAddress(), "uastro", 23456)
}

// now let's see what if the packet times out. the dest relayer fee should be
// refunded to the user (since they packet never reached the dest chain), but
// the src relayer fee should be paid out.
func (suite *testSuite) TestRelayerFeePacketTimedOut() {}
