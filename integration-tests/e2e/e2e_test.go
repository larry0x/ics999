package e2e_test

import (
	"encoding/json"
	"fmt"

	"github.com/stretchr/testify/require"

	sdk "github.com/cosmos/cosmos-sdk/types"

	wasmtypes "github.com/CosmWasm/wasmd/x/wasm/types"
	wasmvmtypes "github.com/CosmWasm/wasmvm/types"

	"ics999/types"
)

// TestRegisterAccount in this test, we do a single action which is to register
// an account. We verify the account contract is instantiated with the correct
// configuration.
func (suite *testSuite) TestRegisterAccount() {
	controller := suite.chainA.SenderAccount.GetAddress().String()

	// invoke ExecuteMsg::Act on chainA with a single action - RegisterAccount
	act(suite, []types.Action{
		{
			RegisterAccount: &types.RegisterAccountAction{},
		},
	})

	// check if an account has been registered
	accountAddr, err := queryAccount(suite.chainB, suite.pathAB.EndpointA.ConnectionID, controller)
	require.NoError(suite.T(), err)

	// query the account contract info
	accountInfo := suite.chainB.ContractInfo(accountAddr)
	require.Equal(suite.T(), suite.chainB.accountCodeID, accountInfo.CodeID)
	require.Equal(suite.T(), suite.chainB.coreAddr.String(), accountInfo.Admin)
	require.Equal(suite.T(), fmt.Sprintf("one-account/%s/%s", suite.pathAB.EndpointB.ConnectionID, controller), accountInfo.Label)

	// query the account contract ownership
	ownershipRes := types.OwnershipResponse{}
	err = suite.chainB.SmartQuery(
		accountAddr.String(),
		types.OwnableQueryMsg{
			Ownership: &types.OwnershipQuery{},
		},
		&ownershipRes,
	)
	require.NoError(suite.T(), err)
	require.Equal(suite.T(), suite.chainB.coreAddr.String(), ownershipRes.Owner)

	// TODO: make sure account cannot be registered twice
}

// TestExecuteWasm in this test, we deploy the mock-counter contract and use the
// interchain account to increment its number.
func (suite *testSuite) TestExecuteWasm() {
	controller := suite.chainA.SenderAccount.GetAddress().String()

	// first, deploy the counter contract
	counter := deployCounter(suite.chainB)

	// test 1 - register account and increment counter once in a single packet
	act(suite, []types.Action{
		{
			RegisterAccount: &types.RegisterAccountAction{},
		},
		{
			Execute: &wasmvmtypes.WasmMsg{
				Execute: &wasmvmtypes.ExecuteMsg{
					ContractAddr: counter.String(),
					Msg:          []byte(`{"increment":{}}`),
					Funds:        wasmvmtypes.Coins{},
				},
			},
		},
	})

	// check if an account has been registered
	_, err := queryAccount(suite.chainB, suite.pathAB.EndpointA.ConnectionID, controller)
	require.NoError(suite.T(), err)

	// check if the number has been correctly incremented once
	number, err := queryNumber(suite.chainB, counter)
	require.NoError(suite.T(), err)
	require.Equal(suite.T(), uint64(1), number)

	// // test 2 - increment the number twice in a single packet
	// act(suite, []types.Action{
	// 	{
	// 		Execute: &wasmvmtypes.WasmMsg{
	// 			Execute: &wasmvmtypes.ExecuteMsg{
	// 				ContractAddr: counter.String(),
	// 				Msg:          []byte(`{"increment":{}}`),
	// 				Funds:        wasmvmtypes.Coins{},
	// 			},
	// 		},
	// 	},
	// 	{
	// 		Execute: &wasmvmtypes.WasmMsg{
	// 			Execute: &wasmvmtypes.ExecuteMsg{
	// 				ContractAddr: counter.String(),
	// 				Msg:          []byte(`{"increment":{}}`),
	// 				Funds:        wasmvmtypes.Coins{},
	// 			},
	// 		},
	// 	},
	// })

	// // check if the number has been correctly incremented two more times
	// number, err = queryNumber(suite.chainB, counter)
	// require.NoError(suite.T(), err)
	// require.Equal(suite.T(), uint64(3), number)
}

// deployCounter deploys the mock-counter contract and returns the address
func deployCounter(chain *testChain) sdk.AccAddress {
	counterStoreRes := chain.StoreCodeFile("../../artifacts/mock_counter-aarch64.wasm")
	return chain.InstantiateContract(counterStoreRes.CodeID, []byte("{}"))
}

// act controller on chainA executes some actions on chainB
func act(suite *testSuite, actions []types.Action) {
	// compose the executeMsg
	executeMsg, err := json.Marshal(types.CoreExecuteMsg{
		Act: &types.Act{
			ConnectionID: suite.pathAB.EndpointA.ConnectionID,
			Actions:      actions,
		},
	})
	require.NoError(suite.T(), err)

	// executes one-core contract on chainA
	_, err = suite.chainA.SendMsgs(&wasmtypes.MsgExecuteContract{
		Sender:   suite.chainA.SenderAccount.GetAddress().String(),
		Contract: suite.chainA.coreAddr.String(),
		Msg:      executeMsg,
		Funds:    []sdk.Coin{},
	})
	require.NoError(suite.T(), err)

	// relay packet to chainB
	err = suite.coordinator.RelayAndAckPendingPackets(suite.pathAB)
	require.NoError(suite.T(), err)
}

// queryAccount queries the account owned by the specified controller
func queryAccount(chain *testChain, connectionID, controller string) (sdk.AccAddress, error) {
	accountRes := types.AccountResponse{}
	if err := chain.SmartQuery(
		chain.coreAddr.String(),
		types.CoreQueryMsg{
			Account: &types.AccountQuery{
				ConnectionID: connectionID,
				Controller:   controller,
			},
		},
		&accountRes,
	); err != nil {
		return nil, err
	}

	accountAddr, err := sdk.AccAddressFromBech32(accountRes.Address)
	if err != nil {
		return nil, err
	}

	return accountAddr, nil
}

// assertNumber check whether the number stored in mock-counter contract equals
// the specified value
func queryNumber(chain *testChain, counter sdk.AccAddress) (uint64, error) {
	numberRes := types.NumberResponse{}
	err := chain.SmartQuery(
		counter.String(),
		&types.CounterQueryMsg{
			Number: &types.NumberQuery{},
		},
		&numberRes,
	)
	return numberRes.Number, err
}
