package e2e_test

import (
	"fmt"

	//lint:ignore SA1019 cosmos-sdk uses deprecated dependency, not my problem
	"github.com/golang/protobuf/proto"
	"github.com/stretchr/testify/require"

	wasmtypes "github.com/CosmWasm/wasmd/x/wasm/types"
	wasmvmtypes "github.com/CosmWasm/wasmvm/types"

	"ics999/tests/types"
)

// TestRegisterAccount in this test, we do a single action which is to register
// an account. We verify the account contract is instantiated with the correct
// configuration.
func (suite *testSuite) TestRegisterAccount() {
	// invoke ExecuteMsg::Act on chainA with a single action - RegisterAccount
	_, ack1, err := act(suite.chainA, suite.pathAB, []types.Action{
		{
			RegisterAccount: &types.RegisterAccountAction{},
		},
	})
	require.NoError(suite.T(), err)
	requirePacketSuccess(suite.T(), ack1)

	// check if an account has been registered, and its address matches that
	// returned in the packet ack
	accountAddr, err := queryAccount(
		suite.chainB,
		suite.pathAB.EndpointA.ChannelID,
		suite.chainA.senderAddr.String(),
	)
	require.NoError(suite.T(), err)
	require.Equal(suite.T(), ack1.Results[0].RegisterAccount.Address, accountAddr.String())

	// query the account contract info
	accountInfo := suite.chainB.ContractInfo(accountAddr)
	require.Equal(suite.T(), suite.chainB.accountCodeID, accountInfo.CodeID)
	require.Equal(suite.T(), suite.chainB.coreAddr.String(), accountInfo.Admin)
	require.Equal(
		suite.T(),
		fmt.Sprintf("one-account/%s/%s", suite.pathAB.EndpointB.ChannelID, suite.chainA.senderAddr.String()),
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

	// attempt to register account again, should fail
	_, ack2, err := act(suite.chainA, suite.pathAB, []types.Action{
		{
			RegisterAccount: &types.RegisterAccountAction{},
		},
	})
	require.NoError(suite.T(), err)
	requirePacketFailed(suite.T(), ack2)
}

// TestExecuteWasm in this test, we deploy the mock-counter contract and use the
// interchain account to increment its number.
func (suite *testSuite) TestExecuteWasm() {
	// test 1 - register account and increment counter once in a single packet
	_, ack1, err := act(suite.chainA, suite.pathAB, []types.Action{
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
	requirePacketSuccess(suite.T(), ack1)

	// check the ack includes the correct result
	res := wasmtypes.MsgExecuteContractResponse{}
	err = proto.Unmarshal(ack1.Results[1].Execute.Data, &res)
	require.NoError(suite.T(), err)
	require.Equal(suite.T(), []byte(`{"new_number":1}`), res.Data)

	// check if the number has been correctly incremented once
	requireNumberEqual(suite.T(), suite.chainB, 1)

	// test 2 - increment the number more times in a single packet
	_, ack2, err := act(suite.chainA, suite.pathAB, []types.Action{
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
	requirePacketSuccess(suite.T(), ack2)

	// check if the number has been correctly incremented two more times
	requireNumberEqual(suite.T(), suite.chainB, 4)
}

func (suite *testSuite) TestQuery() {
	// we query the number (both raw and smart), increase the counter once, then
	// query again
	_, ack, err := act(suite.chainA, suite.pathAB, []types.Action{
		{
			Query: &wasmvmtypes.QueryRequest{
				Wasm: &wasmvmtypes.WasmQuery{
					Raw: &wasmvmtypes.RawQuery{
						ContractAddr: suite.chainB.counterAddr.String(),
						Key:          []byte("number"),
					},
				},
			},
		},
		{
			Query: &wasmvmtypes.QueryRequest{
				Wasm: &wasmvmtypes.WasmQuery{
					Smart: &wasmvmtypes.SmartQuery{
						ContractAddr: suite.chainB.counterAddr.String(),
						Msg:          []byte(`{"number":{}}`),
					},
				},
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
			Query: &wasmvmtypes.QueryRequest{
				Wasm: &wasmvmtypes.WasmQuery{
					Raw: &wasmvmtypes.RawQuery{
						ContractAddr: suite.chainB.counterAddr.String(),
						Key:          []byte("number"),
					},
				},
			},
		},
		{
			Query: &wasmvmtypes.QueryRequest{
				Wasm: &wasmvmtypes.WasmQuery{
					Smart: &wasmvmtypes.SmartQuery{
						ContractAddr: suite.chainB.counterAddr.String(),
						Msg:          []byte(`{"number":{}}`),
					},
				},
			},
		},
	})
	require.NoError(suite.T(), err)
	require.Equal(suite.T(), []byte("0"), ack.Results[0].Query.Response)
	require.Equal(suite.T(), []byte(`{"number":0}`), ack.Results[1].Query.Response)
	require.Equal(suite.T(), []byte("1"), ack.Results[4].Query.Response)
	require.Equal(suite.T(), []byte(`{"number":1}`), ack.Results[5].Query.Response)
}

func (suite *testSuite) TestCallback() {
	// register an account, increment the counter, and query the number
	packet1, ack1, err := act(suite.chainA, suite.pathAB, []types.Action{
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
			Query: &wasmvmtypes.QueryRequest{
				Wasm: &wasmvmtypes.WasmQuery{
					Smart: &wasmvmtypes.SmartQuery{
						ContractAddr: suite.chainB.counterAddr.String(),
						Msg:          []byte(`{"number":{}}`),
					},
				},
			},
		},
	})
	require.NoError(suite.T(), err)
	requirePacketSuccess(suite.T(), ack1)

	// the mock-sender contract should have stored the packet outcome during the
	// callback. let's grab this outcome
	requireOutcomeEqual(suite.T(), suite.chainA, packet1.SourceChannel, packet1.Sequence, "successful")

	// do the same thing but with an intentionally failed packet
	packet2, ack2, err := act(suite.chainA, suite.pathAB, []types.Action{
		{
			Execute: &wasmvmtypes.CosmosMsg{
				Wasm: &wasmvmtypes.WasmMsg{
					Execute: &wasmvmtypes.ExecuteMsg{
						ContractAddr: suite.chainB.counterAddr.String(),
						Msg:          []byte(`{"increment_but_fail":{}}`),
						Funds:        wasmvmtypes.Coins{},
					},
				},
			},
		},
	})
	require.NoError(suite.T(), err)
	requirePacketFailed(suite.T(), ack2)

	// mock-sender should have recorded the correct packet outcome
	requireOutcomeEqual(suite.T(), suite.chainA, packet2.SourceChannel, packet2.Sequence, "failed")
}
