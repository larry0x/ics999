package e2e_test

import (
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"testing"

	"github.com/stretchr/testify/require"
	"github.com/stretchr/testify/suite"

	//lint:ignore SA1019 yeah we use ripemd160
	"golang.org/x/crypto/ripemd160"

	sdk "github.com/cosmos/cosmos-sdk/types"
	channeltypes "github.com/cosmos/ibc-go/v4/modules/core/04-channel/types"
	ibctesting "github.com/cosmos/ibc-go/v4/testing"

	tokenfactorytypes "github.com/CosmWasm/token-factory/x/tokenfactory/types"
	wasmibctesting "github.com/CosmWasm/wasmd/x/wasm/ibctesting"
	wasmtypes "github.com/CosmWasm/wasmd/x/wasm/types"
	wasmvmtypes "github.com/CosmWasm/wasmvm/types"

	"ics999/tests/types"
)

type testSuite struct {
	suite.Suite

	coordinator *wasmibctesting.Coordinator

	chainA *testChain
	chainB *testChain
	chainC *testChain

	pathAB *wasmibctesting.Path
	pathBC *wasmibctesting.Path
}

func (suite *testSuite) SetupTest() {
	suite.coordinator = wasmibctesting.NewCoordinator(suite.T(), 3)

	suite.chainA = setupChain(
		suite.T(),
		suite.coordinator.GetChain(wasmibctesting.GetChainID(0)),
		sdk.NewCoin("uastro", sdk.NewInt(mockInitialBalance)),
		sdk.NewCoin("umars", sdk.NewInt(mockInitialBalance)),
	)
	suite.chainB = setupChain(suite.T(), suite.coordinator.GetChain(wasmibctesting.GetChainID(1)))
	suite.chainC = setupChain(suite.T(), suite.coordinator.GetChain(wasmibctesting.GetChainID(2)))

	suite.pathAB = setupConnection(suite.coordinator, suite.chainA, suite.chainB)
	suite.pathBC = setupConnection(suite.coordinator, suite.chainB, suite.chainC)
}

type testChain struct {
	*wasmibctesting.TestChain

	coreAddr    sdk.AccAddress
	senderAddr  sdk.AccAddress
	counterAddr sdk.AccAddress

	accountCodeID uint64
}

func setupChain(t *testing.T, chain *wasmibctesting.TestChain, coins ...sdk.Coin) *testChain {
	// store contract codes
	//
	// NOTE: wasmd 0.30 uses the gas limit of 3,000,000 for simulation txs.
	// however, our StoreCode txs easily go over this limit. we had to manually
	// increase it. for tests to work.
	// this will no longer be a problem with wasmd 0.31, which uses
	// simtestutil.DefaultGenTxGas which is 10M.
	coreStoreRes := chain.StoreCodeFile("../artifacts/one_core.wasm")
	require.Equal(t, uint64(1), coreStoreRes.CodeID)
	accountStoreRes := chain.StoreCodeFile("../artifacts/one_account.wasm")
	require.Equal(t, uint64(2), accountStoreRes.CodeID)
	senderStoreRes := chain.StoreCodeFile("../artifacts/mock_sender.wasm")
	require.Equal(t, uint64(3), senderStoreRes.CodeID)
	counterStoreRes := chain.StoreCodeFile("../artifacts/mock_counter.wasm")
	require.Equal(t, uint64(4), counterStoreRes.CodeID)

	// instantiate one-core contract
	coreInstantiateMsg, err := json.Marshal(&types.CoreInstantiateMsg{
		AccountCodeID:      accountStoreRes.CodeID,
		DefaultTimeoutSecs: 600, // 10 mins
	})
	require.NoError(t, err)
	core := chain.InstantiateContract(coreStoreRes.CodeID, coreInstantiateMsg)

	// instantiate mock-sender contract
	senderInstantiateMsg, err := json.Marshal(&types.SenderInstantiateMsg{
		OneCore: core.String(),
	})
	require.NoError(t, err)
	sender := chain.InstantiateContract(senderStoreRes.CodeID, senderInstantiateMsg)

	// instantiate mock-counter contract
	counter := chain.InstantiateContract(counterStoreRes.CodeID, []byte("{}"))

	// mint coins to the sender contract
	mintCoinsToAccount(chain, sender, coins...)

	// important: set denom creation fee to zero (default is 10000000stake)
	chain.App.TokenFactoryKeeper.SetParams(chain.GetContext(), tokenfactorytypes.NewParams(sdk.NewCoins()))

	return &testChain{
		TestChain:     chain,
		coreAddr:      core,
		senderAddr:    sender,
		counterAddr:   counter,
		accountCodeID: accountStoreRes.CodeID,
	}
}

func mintCoinsToAccount(chain *wasmibctesting.TestChain, recipient sdk.AccAddress, coins ...sdk.Coin) {
	// the bank keeper only supports minting coins to module accounts
	//
	// in order to mint coins to a base account, we need to mint to a random
	// module account first, then transfer that to the base account
	//
	// this module account must have authtypes.Minter permission in app.go
	randomModuleName := "mint"

	chain.App.BankKeeper.MintCoins(chain.GetContext(), randomModuleName, coins)
	chain.App.BankKeeper.SendCoinsFromModuleToAccount(chain.GetContext(), randomModuleName, recipient, coins)
}

func setupConnection(coordinator *wasmibctesting.Coordinator, chainA, chainB *testChain) *wasmibctesting.Path {
	path := wasmibctesting.NewPath(chainA.TestChain, chainB.TestChain)
	path.EndpointA.ChannelConfig = &ibctesting.ChannelConfig{
		PortID:  chainA.ContractInfo(chainA.coreAddr).IBCPortID,
		Order:   types.Order,
		Version: types.Version,
	}
	path.EndpointB.ChannelConfig = &ibctesting.ChannelConfig{
		PortID:  chainB.ContractInfo(chainB.coreAddr).IBCPortID,
		Order:   types.Order,
		Version: types.Version,
	}

	coordinator.SetupConnections(path)
	coordinator.CreateChannels(path)

	return path
}

// relaySinglePacket relays a single packet from EndpointA to EndpointB.
// To relayer a packet from B to A, do: relaySinglePacket(reversePath(path)).
//
// We choose to write our own relaying instead of using coordinator.RelayAndAckPendingPackets
// because we want to grab the original packet and ack and assert their contents
// are correct
func relaySinglePacket(path *wasmibctesting.Path) (*channeltypes.Packet, []byte, error) {
	// in this function, we relay from EndpointA --> EndpointB
	src := path.EndpointA
	dest := path.EndpointB

	if len(src.Chain.PendingSendPackets) < 1 {
		return nil, nil, errors.New("no packet to relay")
	}

	// grab the first pending packet
	packet := src.Chain.PendingSendPackets[0]
	src.Chain.PendingSendPackets = src.Chain.PendingSendPackets[1:]

	if err := dest.UpdateClient(); err != nil {
		return nil, nil, err
	}

	res, err := dest.RecvPacketWithResult(packet)
	if err != nil {
		return nil, nil, err
	}

	// print out the events for debugging purpose
	// TODO: delete this
	events := res.GetEvents()
	for _, event := range events {
		fmt.Println("event_type:", event.Type)
		for _, attr := range event.Attributes {
			fmt.Println(" - key:", string(attr.Key))
			fmt.Println("   value:", string(attr.Value))
		}
	}

	ack, err := ibctesting.ParseAckFromEvents(res.GetEvents())
	if err != nil {
		return nil, nil, err
	}

	if err = src.AcknowledgePacket(packet, ack); err != nil {
		return nil, nil, err
	}

	return &packet, ack, err
}

// reversePath change the order of EndpointA and EndpointB in a path
//
//lint:ignore U1000 will be used later
func reversePath(path *wasmibctesting.Path) *wasmibctesting.Path {
	return &wasmibctesting.Path{
		EndpointA: path.EndpointB,
		EndpointB: path.EndpointA,
	}
}

func act(src *testChain, path *wasmibctesting.Path, actions []types.Action) (*channeltypes.Packet, *types.PacketAck, error) {
	// compose the executeMsg
	executeMsg, err := json.Marshal(types.SenderExecuteMsg{
		Send: &types.Send{
			ConnectionID: path.EndpointA.ConnectionID,
			Actions:      actions,
		},
	})
	if err != nil {
		return nil, nil, err
	}

	// executes mock-sender contract on chainA
	if _, err = src.SendMsgs(&wasmtypes.MsgExecuteContract{
		Sender:   src.SenderAccount.GetAddress().String(),
		Contract: src.senderAddr.String(),
		Msg:      executeMsg,
		Funds:    []sdk.Coin{},
	}); err != nil {
		return nil, nil, err
	}

	// relay the packet
	packet, ackBytes, err := relaySinglePacket(path)
	if err != nil {
		return nil, nil, err
	}

	ack := &types.PacketAck{}
	if err = json.Unmarshal(ackBytes, ack); err != nil {
		return nil, nil, err
	}

	return packet, ack, nil
}

func queryAccount(chain *testChain, channelID, controller string) (sdk.AccAddress, error) {
	accountRes := types.AccountResponse{}
	if err := chain.SmartQuery(
		chain.coreAddr.String(),
		types.CoreQueryMsg{
			Account: &types.AccountQuery{
				ChannelID:  channelID,
				Controller: controller,
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

func requirePacketSuccess(t *testing.T, ack *types.PacketAck) {
	require.NotEmpty(t, ack.Results)
	require.Empty(t, ack.Error)
}

func requirePacketFailed(t *testing.T, ack *types.PacketAck) {
	require.Empty(t, ack.Results)
	require.NotEmpty(t, ack.Error)
}

func requireBalanceEqual(t *testing.T, chain *testChain, addr sdk.AccAddress, denom string, expBalance int64) {
	balance := chain.Balance(addr, denom)
	require.Equal(t, sdk.NewInt(expBalance), balance.Amount)
}

func requireTraceEqual(t *testing.T, chain *testChain, denom string, expTrace types.Trace) {
	traceResp := types.DenomTraceResponse{}
	err := chain.SmartQuery(
		chain.coreAddr.String(),
		&types.CoreQueryMsg{
			DenomTrace: &types.DenomTraceQuery{
				Denom: denom,
			},
		},
		&traceResp,
	)
	require.NoError(t, err)
	require.Equal(t, expTrace.Denom, traceResp.Denom)
	require.Equal(t, expTrace.BaseDenom, traceResp.BaseDenom)
	require.Equal(t, expTrace.Path, traceResp.Path)
}

func requireNumberEqual(t *testing.T, chain *testChain, expNumber uint64) {
	numberRes := types.NumberResponse{}
	err := chain.SmartQuery(
		chain.counterAddr.String(),
		&types.CounterQueryMsg{
			Number: &types.NumberQuery{},
		},
		&numberRes,
	)
	require.NoError(t, err)
	require.Equal(t, expNumber, numberRes.Number)
}

func requireOutcomeEqual(t *testing.T, chain *testChain, channelID string, sequence uint64, expOutcome string) {
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
	require.NoError(t, err)
	require.Equal(t, expOutcome, outcomeRes.Outcome)
}

func Test(t *testing.T) {
	suite.Run(t, new(testSuite))
}
