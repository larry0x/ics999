package e2e_test

import (
	"encoding/hex"
	"fmt"
	"testing"

	//lint:ignore SA1019 yeah we use ripemd160
	"golang.org/x/crypto/ripemd160"

	"github.com/stretchr/testify/require"

	sdk "github.com/cosmos/cosmos-sdk/types"

	wasmibctesting "github.com/CosmWasm/wasmd/x/wasm/ibctesting"
	wasmvmtypes "github.com/CosmWasm/wasmvm/types"

	"ics999/tests/types"
)

var mockRecipient, _ = sdk.AccAddressFromBech32("cosmos1z926ax906k0ycsuckele6x5hh66e2m4mjchwmp")

// TestTransfer tests sending multiple coins to multiple recipients in a single
// packet. We want to make sure the denom is only created once.
func (suite *testSuite) TestTransfer() {
	// the first two transfers we specify a recipient
	// the other two we don't specify a recipient; should default to the ICA
	_, ack, err := act(suite, []types.Action{
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
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.senderAddr, "uastro", 100_000_000-888_888-987_654)
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.senderAddr, "umars", 100_000_000-69_420-1_111_111)
	requireBalanceEqual(suite.T(), suite.chainB, mockRecipient, astroVoucherDenom, 888_888)
	requireBalanceEqual(suite.T(), suite.chainB, mockRecipient, marsVoucherDenom, 69_420)
	requireBalanceEqual(suite.T(), suite.chainB, icaAddr, astroVoucherDenom, 987_654)
	requireBalanceEqual(suite.T(), suite.chainB, icaAddr, marsVoucherDenom, 1_111_111)
}

// TestPathUnwinding in this test, to do the following transfer:
//
//	chainA --> chainB --> chainC --> chainB
//
// The objective is to test in the last step, whether the voucher tokens are
// properly burned and escrowed tokens released.
func (suite *testSuite) TestPathUnwinding() {
	// todo
}

// TestRefund tests the funds escrowed on the sender chain is properly refunded
// if the packet fails to execute.
func (suite *testSuite) TestRefund() {
	// todo
}

// TestSwap the most complex test - we send coins from chainA to chainB, make a
// swap at a DEX contract on chainB, then send the proceedings back to chainA,
// all in the same packet.
func (suite *testSuite) TestSwap() {
	// todo
}

func deriveVoucherDenom(chain *testChain, testPaths []*wasmibctesting.Path, baseDenom string) string {
	// convert ibctesting.Endpoint to wasmvmtypes.IBCEndpoint
	path := []wasmvmtypes.IBCEndpoint{}
	for _, testPath := range testPaths {
		path = append(path, wasmvmtypes.IBCEndpoint{
			PortID:    testPath.EndpointB.ChannelConfig.PortID,
			ChannelID: testPath.EndpointB.ChannelID,
		})
	}

	denomHash := denomHashFromTrace(types.Trace{
		BaseDenom: baseDenom,
		Path:      path,
	})

	return fmt.Sprintf("factory/%s/%s", chain.coreAddr, denomHash)
}

func denomHashFromTrace(trace types.Trace) string {
	hasher := ripemd160.New()

	hasher.Write([]byte(trace.BaseDenom))

	for _, step := range trace.Path {
		hasher.Write([]byte(step.PortID))
		hasher.Write([]byte(step.ChannelID))
	}

	return hex.EncodeToString(hasher.Sum(nil))
}

func requireBalanceEqual(t *testing.T, chain *testChain, addr sdk.AccAddress, denom string, expBalance int64) {
	balance := chain.Balance(addr, denom)
	require.Equal(t, sdk.NewInt(expBalance), balance.Amount)
}
