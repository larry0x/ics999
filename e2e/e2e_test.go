package e2e_test

import (
	"encoding/json"
	"fmt"

	//lint:ignore SA1019 cosmos-sdk uses deprecated dependency, not my problem
	"github.com/golang/protobuf/proto"
	"github.com/stretchr/testify/require"

	sdk "github.com/cosmos/cosmos-sdk/types"
	channeltypes "github.com/cosmos/ibc-go/v6/modules/core/04-channel/types"

	wasmtypes "github.com/CosmWasm/wasmd/x/wasm/types"
	wasmvmtypes "github.com/CosmWasm/wasmvm/types"

	"ics999/e2e/types"
)

// TestRegisterAccount in this test, we do a single action which is to register
// an account. We verify the account contract is instantiated with the correct
// configuration.
func (suite *testSuite) TestRegisterAccount() {
	// invoke ExecuteMsg::Act on chainA with a single action - RegisterAccount
	_, ack, err := act(suite, []types.Action{
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
		suite.chainA.senderAddr.String(),
	)
	require.NoError(suite.T(), err)
	require.Equal(suite.T(), ack.Result[0].RegisterAccount.Address, accountAddr.String())

	// query the account contract info
	accountInfo := suite.chainB.ContractInfo(accountAddr)
	require.Equal(suite.T(), suite.chainB.accountCodeID, accountInfo.CodeID)
	require.Equal(suite.T(), suite.chainB.coreAddr.String(), accountInfo.Admin)
	require.Equal(
		suite.T(),
		fmt.Sprintf("one-account/%s/%s", suite.pathAB.EndpointB.ConnectionID, suite.chainA.senderAddr.String()),
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
	_, ack, err := act(suite, []types.Action{
		{
			RegisterAccount: &types.RegisterAccountAction{},
		},
		{
			Execute: &wasmvmtypes.WasmMsg{
				Execute: &wasmvmtypes.ExecuteMsg{
					ContractAddr: suite.chainB.counterAddr.String(),
					Msg:          []byte(`{"increment":{}}`),
					Funds:        wasmvmtypes.Coins{},
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
	number, err := queryNumber(suite.chainB)
	require.NoError(suite.T(), err)
	require.Equal(suite.T(), uint64(1), number)

	// test 2 - increment the number more times in a single packet
	_, _, err = act(suite, []types.Action{
		{
			Execute: &wasmvmtypes.WasmMsg{
				Execute: &wasmvmtypes.ExecuteMsg{
					ContractAddr: suite.chainB.counterAddr.String(),
					Msg:          []byte(`{"increment":{}}`),
					Funds:        wasmvmtypes.Coins{},
				},
			},
		},
		{
			Execute: &wasmvmtypes.WasmMsg{
				Execute: &wasmvmtypes.ExecuteMsg{
					ContractAddr: suite.chainB.counterAddr.String(),
					Msg:          []byte(`{"increment":{}}`),
					Funds:        wasmvmtypes.Coins{},
				},
			},
		},
		{
			Execute: &wasmvmtypes.WasmMsg{
				Execute: &wasmvmtypes.ExecuteMsg{
					ContractAddr: suite.chainB.counterAddr.String(),
					Msg:          []byte(`{"increment":{}}`),
					Funds:        wasmvmtypes.Coins{},
				},
			},
		},
	})
	require.NoError(suite.T(), err)

	// check if the number has been correctly incremented two more times
	number, err = queryNumber(suite.chainB)
	require.NoError(suite.T(), err)
	require.Equal(suite.T(), uint64(4), number)
}

func (suite *testSuite) TestQuery() {
	// we query the number (both raw and smart), increase the counter once, then
	// query again
	_, ack, err := act(suite, []types.Action{
		{
			Query: &wasmvmtypes.WasmQuery{
				Raw: &wasmvmtypes.RawQuery{
					ContractAddr: suite.chainB.counterAddr.String(),
					Key:          []byte("number"),
				},
			},
		},
		{
			Query: &wasmvmtypes.WasmQuery{
				Smart: &wasmvmtypes.SmartQuery{
					ContractAddr: suite.chainB.counterAddr.String(),
					Msg:          []byte(`{"number":{}}`),
				},
			},
		},
		{
			RegisterAccount: &types.RegisterAccountAction{},
		},
		{
			Execute: &wasmvmtypes.WasmMsg{
				Execute: &wasmvmtypes.ExecuteMsg{
					ContractAddr: suite.chainB.counterAddr.String(),
					Msg:          []byte(`{"increment":{}}`),
					Funds:        wasmvmtypes.Coins{},
				},
			},
		},
		{
			Query: &wasmvmtypes.WasmQuery{
				Raw: &wasmvmtypes.RawQuery{
					ContractAddr: suite.chainB.counterAddr.String(),
					Key:          []byte("number"),
				},
			},
		},
		{
			Query: &wasmvmtypes.WasmQuery{
				Smart: &wasmvmtypes.SmartQuery{
					ContractAddr: suite.chainB.counterAddr.String(),
					Msg:          []byte(`{"number":{}}`),
				},
			},
		},
	})
	require.NoError(suite.T(), err)
	require.Equal(suite.T(), []byte("0"), ack.Result[0].Query.Response)
	require.Equal(suite.T(), []byte(`{"number":0}`), ack.Result[1].Query.Response)
	require.Equal(suite.T(), []byte("1"), ack.Result[4].Query.Response)
	require.Equal(suite.T(), []byte(`{"number":1}`), ack.Result[5].Query.Response)
}

func (suite *testSuite) TestCallback() {
	// register an account, increment the counter, and query the number
	packet1, ack1, err := act(suite, []types.Action{
		{
			RegisterAccount: &types.RegisterAccountAction{},
		},
		{
			Execute: &wasmvmtypes.WasmMsg{
				Execute: &wasmvmtypes.ExecuteMsg{
					ContractAddr: suite.chainB.counterAddr.String(),
					Msg:          []byte(`{"increment":{}}`),
					Funds:        wasmvmtypes.Coins{},
				},
			},
		},
		{
			Query: &wasmvmtypes.WasmQuery{
				Smart: &wasmvmtypes.SmartQuery{
					ContractAddr: suite.chainB.counterAddr.String(),
					Msg:          []byte(`{"number":{}}`),
				},
			},
		},
	})
	require.NoError(suite.T(), err)

	// make sure the actions are successful
	// this is a weird way to match enum variants, but it works
	require.NotEmpty(suite.T(), ack1.Result)
	require.Empty(suite.T(), ack1.Error)

	// the mock-sender contract should have stored the packet outcome during the
	// callback. let's grab this outcome
	outcome1, _ := queryOutcome(suite.chainA, packet1.SourceChannel, packet1.Sequence)
	require.NoError(suite.T(), err)
	require.Equal(suite.T(), "successful", outcome1)

	// do the same thing but with an intentionally failed packet
	packet2, ack2, err := act(suite, []types.Action{
		{
			Execute: &wasmvmtypes.WasmMsg{
				Execute: &wasmvmtypes.ExecuteMsg{
					ContractAddr: suite.chainB.counterAddr.String(),
					Msg:          []byte(`{"increment_but_fail":{}}`),
					Funds:        wasmvmtypes.Coins{},
				},
			},
		},
	})
	require.NoError(suite.T(), err)

	// make sure the action indeed failed
	require.Empty(suite.T(), ack2.Result)
	require.NotEmpty(suite.T(), ack2.Error)

	// mock-sender should have recorded the correct packet outcome
	outcome2, _ := queryOutcome(suite.chainA, packet2.SourceChannel, packet2.Sequence)
	require.NoError(suite.T(), err)
	require.Equal(suite.T(), "failed", outcome2)
}

// act controller on chainA executes some actions on chainB
func act(suite *testSuite, actions []types.Action) (*channeltypes.Packet, *types.PacketAck, error) {
	// compose the executeMsg
	executeMsg, err := json.Marshal(types.SenderExecuteMsg{
		Send: &types.Send{
			ConnectionID: suite.pathAB.EndpointA.ConnectionID,
			Actions:      actions,
		},
	})
	if err != nil {
		return nil, nil, err
	}

	// executes mock-sender contract on chainA
	if _, err = suite.chainA.SendMsgs(&wasmtypes.MsgExecuteContract{
		Sender:   suite.chainA.SenderAccount.GetAddress().String(),
		Contract: suite.chainA.senderAddr.String(),
		Msg:      executeMsg,
		Funds:    []sdk.Coin{},
	}); err != nil {
		return nil, nil, err
	}

	// relay the packet
	packet, ackBytes, err := relaySinglePacket(suite.pathAB)
	if err != nil {
		return nil, nil, err
	}

	ack := &types.PacketAck{}
	if err = json.Unmarshal(ackBytes, ack); err != nil {
		return nil, nil, err
	}

	return packet, ack, nil
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

// queryOutcome queries the mock-sender contract for the outcome of a packet
// that it stores
func queryOutcome(chain *testChain, channelID string, sequence uint64) (string, error) {
	outcomeRes := types.OutcomeResponse{}
	err := chain.SmartQuery(
		chain.senderAddr.String(),
		&types.SenderQueryMsg{
			Outcome: &types.OutcomeQuery{
				ChannelID: channelID,
				Sequence:  sequence,
			},
		},
		&outcomeRes,
	)
	return outcomeRes.Outcome, err
}

// queryNumber queries the mock-counter contract for the number it stores
func queryNumber(chain *testChain) (uint64, error) {
	numberRes := types.NumberResponse{}
	err := chain.SmartQuery(
		chain.counterAddr.String(),
		&types.CounterQueryMsg{
			Number: &types.NumberQuery{},
		},
		&numberRes,
	)
	return numberRes.Number, err
}
