package e2e_test

import (
	"encoding/json"
	"errors"
	"testing"

	"github.com/stretchr/testify/require"
	"github.com/stretchr/testify/suite"

	sdk "github.com/cosmos/cosmos-sdk/types"

	channeltypes "github.com/cosmos/ibc-go/v6/modules/core/04-channel/types"
	ibctesting "github.com/cosmos/ibc-go/v6/testing"

	wasmibctesting "github.com/CosmWasm/wasmd/x/wasm/ibctesting"

	"ics999/e2e/types"
)

type testSuite struct {
	suite.Suite

	coordinator *wasmibctesting.Coordinator

	chainA *testChain
	chainB *testChain

	pathAB *wasmibctesting.Path
}

func (suite *testSuite) SetupTest() {
	suite.coordinator = wasmibctesting.NewCoordinator(suite.T(), 2)

	suite.chainA = setupChain(suite.T(), suite.coordinator.GetChain(wasmibctesting.GetChainID(0)))
	suite.chainB = setupChain(suite.T(), suite.coordinator.GetChain(wasmibctesting.GetChainID(1)))

	suite.pathAB = setupConnection(suite.coordinator, suite.chainA, suite.chainB)
}

type testChain struct {
	*wasmibctesting.TestChain

	coreAddr    sdk.AccAddress
	senderAddr  sdk.AccAddress
	counterAddr sdk.AccAddress

	accountCodeID uint64
}

func setupChain(t *testing.T, chain *wasmibctesting.TestChain) *testChain {
	// store contract codes
	coreStoreRes := chain.StoreCodeFile("../artifacts/one_core-aarch64.wasm")
	require.Equal(t, uint64(1), coreStoreRes.CodeID)
	accountStoreRes := chain.StoreCodeFile("../artifacts/one_account-aarch64.wasm")
	require.Equal(t, uint64(2), accountStoreRes.CodeID)
	senderStoreRes := chain.StoreCodeFile("../artifacts/mock_sender-aarch64.wasm")
	require.Equal(t, uint64(3), senderStoreRes.CodeID)
	counterStoreRes := chain.StoreCodeFile("../artifacts/mock_counter-aarch64.wasm")
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

	return &testChain{
		TestChain:     chain,
		coreAddr:      core,
		senderAddr:    sender,
		counterAddr:   counter,
		accountCodeID: accountStoreRes.CodeID,
	}
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

	if len(src.Chain.PendingSendPackets) < 1 {
		return nil, nil, errors.New("no packet to relay")
	}

	// grab the first pending packet
	packet := src.Chain.PendingSendPackets[0]
	src.Chain.PendingSendPackets = src.Chain.PendingSendPackets[1:]

	if err := path.EndpointB.UpdateClient(); err != nil {
		return nil, nil, err
	}

	res, err := path.EndpointB.RecvPacketWithResult(packet)
	if err != nil {
		return nil, nil, err
	}

	ack, err := ibctesting.ParseAckFromEvents(res.GetEvents())
	if err != nil {
		return nil, nil, err
	}

	if err = path.EndpointA.AcknowledgePacket(packet, ack); err != nil {
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

func Test(t *testing.T) {
	suite.Run(t, new(testSuite))
}
