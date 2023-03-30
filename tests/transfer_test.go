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

// TestTransfer in this test, we make a single transfer from chainA to chainB
// with the recipient specified.
func (suite *testSuite) TestTransfer() {
	_, ack, err := act(suite, []types.Action{
		{
			Transfer: &types.TransferAction{
				Denom:     "uastro",
				Amount:    sdk.NewInt(69_000_000),
				Recipient: mockRecipient.String(),
			},
		},
	})
	require.NoError(suite.T(), err)
	requirePacketSuccess(suite.T(), ack)

	// predict what the denom would be
	denom := deriveVoucherDenom(suite.chainB, []*wasmibctesting.Path{suite.pathAB}, "uastro")

	// sender balance on chainA should have been reduced
	// recipient balance on chainB should have been increased
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.senderAddr, "uastro", 100_000_000-69_000_000)
	requireBalanceEqual(suite.T(), suite.chainB, mockRecipient, denom, 69_000_000)
}

// TestTransferNoRecipient in this test, we make a single transfer from chainA
// to chainB but without specifying the recipient. The tokens will be sent to
// the sender's interchain account by default. If the sender does not already
// own an ICA, the packet fails.
func (suite *testSuite) TestTransferNoRecipient() {
	_, ack, err := act(suite, []types.Action{
		{
			RegisterAccount: &types.RegisterAccountAction{},
		},
		{
			Transfer: &types.TransferAction{
				Denom:  "umars",
				Amount: sdk.NewInt(123_456),
			},
		},
	})
	require.NoError(suite.T(), err)
	requirePacketSuccess(suite.T(), ack)

	// predict what the denom would be
	denom := deriveVoucherDenom(suite.chainB, []*wasmibctesting.Path{suite.pathAB}, "umars")

	// recipient unspecified, default to the ICA
	ica, err := sdk.AccAddressFromBech32(ack.Results[0].RegisterAccount.Address)
	require.NoError(suite.T(), err)

	// sender balance on chainA should have been reduced
	// recipient balance on chainB should have been increased
	requireBalanceEqual(suite.T(), suite.chainA, suite.chainA.senderAddr, "umars", 100_000_000-123_456)
	requireBalanceEqual(suite.T(), suite.chainB, ica, denom, 123_456)
}

// TestMultipleCoinsOnePacket tests sending multiple coins to multiple
// recipients in a single packet. We want to make sure the denom is only created
// once.
func (suite *testSuite) TestMultipleTransfersPacket() {
	// todo
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
