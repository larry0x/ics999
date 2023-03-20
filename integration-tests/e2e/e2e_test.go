package e2e_test

import (
	"encoding/json"
	"fmt"

	"github.com/stretchr/testify/require"

	sdk "github.com/cosmos/cosmos-sdk/types"

	wasmvmtypes "github.com/CosmWasm/wasmd/x/wasm/types"

	"ics999/types"
)

func (suite *testSuite) TestRegisterAccount() {
	controller := suite.chainA.SenderAccount.GetAddress().String()

	// invoke ExecuteMsg::Act on chainA with a single action - RegisterAccount
	executeMsg, err := json.Marshal(types.CoreExecuteMsg{
		Act: &types.Act{
			ConnectionID: suite.pathAB.EndpointA.ConnectionID,
			Actions: []types.Action{
				{
					RegisterAccount: &types.RegisterAccountAction{},
				},
			},
		},
	})
	require.NoError(suite.T(), err)

	_, err = suite.chainA.SendMsgs(&wasmvmtypes.MsgExecuteContract{
		Sender:   controller,
		Contract: suite.chainA.coreAddr.String(),
		Msg:      executeMsg,
		Funds:    []sdk.Coin{},
	})
	require.NoError(suite.T(), err)

	err = suite.coordinator.RelayAndAckPendingPackets(suite.pathAB)
	require.NoError(suite.T(), err)

	// check if an account has been registered
	accountRes := types.AccountResponse{}
	err = suite.chainB.SmartQuery(
		suite.chainB.coreAddr.String(),
		types.CoreQueryMsg{
			Account: &types.AccountQuery{
				ConnectionID: suite.pathAB.EndpointA.ConnectionID,
				Controller:   controller,
			},
		},
		&accountRes,
	)
	require.NoError(suite.T(), err)

	accountAddr, err := sdk.AccAddressFromBech32(accountRes.Address)
	require.NoError(suite.T(), err)

	// query the account contract info
	accountInfo := suite.chainB.ContractInfo(accountAddr)
	require.Equal(suite.T(), suite.chainB.accountCodeID, accountInfo.CodeID)
	require.Equal(suite.T(), suite.chainB.coreAddr.String(), accountInfo.Admin)
	require.Equal(suite.T(), fmt.Sprintf("one-account/%s/%s", suite.pathAB.EndpointB.ConnectionID, controller), accountInfo.Label)

	// query the account contract ownership
	ownershipRes := types.OwnershipResponse{}
	err = suite.chainB.SmartQuery(
		accountRes.Address,
		types.OwnableQueryMsg{
			Ownership: &types.OwnershipQuery{},
		},
		&ownershipRes,
	)
	require.NoError(suite.T(), err)
	require.Equal(suite.T(), suite.chainB.coreAddr.String(), ownershipRes.Owner)
}
