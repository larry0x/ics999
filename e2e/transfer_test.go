package e2e_test

import (
	sdk "github.com/cosmos/cosmos-sdk/types"

	"github.com/stretchr/testify/require"

	"ics999/e2e/types"
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

	// sender balance on chainA should have been reduced
	senderBalance := suite.chainA.Balance(suite.chainA.senderAddr, "uastro")
	require.Equal(suite.T(), sdk.NewInt(31_000_000), senderBalance.Amount)

	// recipient balance on chainB should have been increased
	recipientBalance := suite.chainB.Balance(mockRecipient, "uastro")
	require.Equal(suite.T(), sdk.NewInt(69_000_000), recipientBalance.Amount)
}

// TestTransferNoRecipient in this test, we make a single transfer from chainA
// to chainB but without specifying the recipient. The tokens will be sent to
// the sender's interchain account by default. If the sender does not already
// own an ICA, the packet fails.
func (suite *testSuite) TestTransferNoRecipient() {
	// todo
}

// TestMultipleCoinsOnePacket tests sending multiple coins to multiple
// recipients in a single packet.
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
