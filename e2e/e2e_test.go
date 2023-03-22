package e2e_test

import (
	"encoding/json"
	"fmt"

	//lint:ignore SA1019 cosmos-sdk uses deprecated dependency, not my problem
	"github.com/golang/protobuf/proto"
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
	// invoke ExecuteMsg::Act on chainA with a single action - RegisterAccount
	ack, err := act(suite, []types.Action{
		{
			RegisterAccount: &types.RegisterAccountAction{},
		},
	})
	require.NoError(suite.T(), err)

	// check if an account has been registered, and its address matches that
	// returned in the packet ack
	accountAddr, err := queryAccount(
		suite.chainB,
		suite.pathAB.EndpointA.ConnectionID,
		suite.chainA.SenderAccount.GetAddress().String(),
	)
	require.NoError(suite.T(), err)
	require.Equal(suite.T(), ack.Result[0].RegisterAccount.Address, accountAddr.String())

	// query the account contract info
	accountInfo := suite.chainB.ContractInfo(accountAddr)
	require.Equal(suite.T(), suite.chainB.accountCodeID, accountInfo.CodeID)
	require.Equal(suite.T(), suite.chainB.coreAddr.String(), accountInfo.Admin)
	require.Equal(
		suite.T(),
		fmt.Sprintf("one-account/%s/%s", suite.pathAB.EndpointB.ConnectionID, suite.chainA.SenderAccount.GetAddress().String()),
		accountInfo.Label,
	)

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
	// test 1 - register account and increment counter once in a single packet
	ack, err := act(suite, []types.Action{
		{
			RegisterAccount: &types.RegisterAccountAction{},
		},
		{
			Execute: &wasmvmtypes.CosmosMsg{
				Wasm: &wasmvmtypes.WasmMsg{
					Execute: &wasmvmtypes.ExecuteMsg{
						ContractAddr: suite.chainB.counterAddr.String(),
						Msg:          []byte(`{"increment":{}}`),
						Funds:        wasmvmtypes.Coins{},
					},
				},
			},
		},
	})
	require.NoError(suite.T(), err)

	// check the ack includes the correct result
	res := wasmtypes.MsgExecuteContractResponse{}
	err = proto.Unmarshal(ack.Result[1].Execute.Data, &res)
	require.NoError(suite.T(), err)
	require.Equal(suite.T(), []byte(`{"new_number":1}`), res.Data)

	// check if the number has been correctly incremented once
	number, err := queryNumber(suite.chainB, suite.chainB.counterAddr)
	require.NoError(suite.T(), err)
	require.Equal(suite.T(), uint64(1), number)

	// test 2 - increment the number more times in a single packet
	_, err = act(suite, []types.Action{
		{
			Execute: &wasmvmtypes.CosmosMsg{
				Wasm: &wasmvmtypes.WasmMsg{
					Execute: &wasmvmtypes.ExecuteMsg{
						ContractAddr: suite.chainB.counterAddr.String(),
						Msg:          []byte(`{"increment":{}}`),
						Funds:        wasmvmtypes.Coins{},
					},
				},
			},
		},
		{
			Execute: &wasmvmtypes.CosmosMsg{
				Wasm: &wasmvmtypes.WasmMsg{
					Execute: &wasmvmtypes.ExecuteMsg{
						ContractAddr: suite.chainB.counterAddr.String(),
						Msg:          []byte(`{"increment":{}}`),
						Funds:        wasmvmtypes.Coins{},
					},
				},
			},
		},
		{
			Execute: &wasmvmtypes.CosmosMsg{
				Wasm: &wasmvmtypes.WasmMsg{
					Execute: &wasmvmtypes.ExecuteMsg{
						ContractAddr: suite.chainB.counterAddr.String(),
						Msg:          []byte(`{"increment":{}}`),
						Funds:        wasmvmtypes.Coins{},
					},
				},
			},
		},
	})
	require.NoError(suite.T(), err)

	// check if the number has been correctly incremented two more times
	number, err = queryNumber(suite.chainB, suite.chainB.counterAddr)
	require.NoError(suite.T(), err)
	require.Equal(suite.T(), uint64(4), number)
}

func (suite *testSuite) TestQuery() {
	// we query the number (both raw and smart), increase the counter once, then
	// query again
	ack, err := act(suite, []types.Action{
		{
			QueryRaw: &types.QueryRawAction{
				Contract: suite.chainB.counterAddr.String(),
				Key:      []byte("number"),
			},
		},
		{
			QuerySmart: &types.QuerySmartAction{
				Contract: suite.chainB.counterAddr.String(),
				Msg:      []byte(`{"number":{}}`),
			},
		},
		{
			RegisterAccount: &types.RegisterAccountAction{},
		},
		{
			Execute: &wasmvmtypes.CosmosMsg{
				Wasm: &wasmvmtypes.WasmMsg{
					Execute: &wasmvmtypes.ExecuteMsg{
						ContractAddr: suite.chainB.counterAddr.String(),
						Msg:          []byte(`{"increment":{}}`),
						Funds:        wasmvmtypes.Coins{},
					},
				},
			},
		},
		{
			QueryRaw: &types.QueryRawAction{
				Contract: suite.chainB.counterAddr.String(),
				Key:      []byte("number"),
			},
		},
		{
			QuerySmart: &types.QuerySmartAction{
				Contract: suite.chainB.counterAddr.String(),
				Msg:      []byte(`{"number":{}}`),
			},
		},
	})
	require.NoError(suite.T(), err)
	require.Equal(suite.T(), []byte("0"), ack.Result[0].QueryRaw.Value)
	require.Equal(suite.T(), []byte(`{"number":0}`), ack.Result[1].QuerySmart.Response)
	require.Equal(suite.T(), []byte("1"), ack.Result[4].QueryRaw.Value)
	require.Equal(suite.T(), []byte(`{"number":1}`), ack.Result[5].QuerySmart.Response)
}

// act controller on chainA executes some actions on chainB
func act(suite *testSuite, actions []types.Action) (*types.PacketAck, error) {
	// compose the executeMsg
	executeMsg, err := json.Marshal(types.CoreExecuteMsg{
		Act: &types.Act{
			ConnectionID: suite.pathAB.EndpointA.ConnectionID,
			Actions:      actions,
		},
	})
	if err != nil {
		return nil, err
	}

	// executes one-core contract on chainA
	if _, err = suite.chainA.SendMsgs(&wasmtypes.MsgExecuteContract{
		Sender:   suite.chainA.SenderAccount.GetAddress().String(),
		Contract: suite.chainA.coreAddr.String(),
		Msg:      executeMsg,
		Funds:    []sdk.Coin{},
	}); err != nil {
		return nil, err
	}

	// relay the packet
	ackBytes, err := relaySinglePacket(suite.pathAB)
	if err != nil {
		return nil, err
	}

	ack := &types.PacketAck{}
	if err = json.Unmarshal(ackBytes, ack); err != nil {
		return nil, err
	}

	return ack, nil
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
