package e2e_test

import (
	"encoding/json"
	"errors"
	"fmt"
	"testing"

	"github.com/stretchr/testify/require"
	"github.com/stretchr/testify/suite"

	sdk "github.com/cosmos/cosmos-sdk/types"

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
	counterAddr sdk.AccAddress

	accountCodeID uint64
}

func setupChain(t *testing.T, chain *wasmibctesting.TestChain) *testChain {
	// store one-core contract code
	coreStoreRes := chain.StoreCodeFile("../../artifacts/one_core.wasm")
	require.Equal(t, uint64(1), coreStoreRes.CodeID)

	// store one-account contract code
	accountStoreRes := chain.StoreCodeFile("../../artifacts/one_account.wasm")
	require.Equal(t, uint64(2), accountStoreRes.CodeID)

	// store mock-counter contract code
	counterStoreRes := chain.StoreCodeFile("../../artifacts/mock_counter.wasm")
	require.Equal(t, uint64(3), counterStoreRes.CodeID)

	// instantiate one-core contract
	instantiateMsg, err := json.Marshal(&types.CoreInstantiateMsg{
		AccountCodeID:  accountStoreRes.CodeID,
		TransferCodeID: uint64(0), // FIXME: placeholder
	})
	require.NoError(t, err)
	core := chain.InstantiateContract(coreStoreRes.CodeID, instantiateMsg)

	// instantiate mock-counter contract
	counter := chain.InstantiateContract(counterStoreRes.CodeID, []byte("{}"))

	return &testChain{
		TestChain:     chain,
		coreAddr:      core,
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
//
// We choose to write our own relaying instead of using coordinator.RelayAndAckPendingPackets
// because:
// - we want to grab the ack and assert its content is correct
// - we want to add some logging capability
func relaySinglePacket(path *wasmibctesting.Path) (ack []byte, err error) {
	// in this function, we relay from EndpointA --> EndpointB
	src := path.EndpointA

	if len(src.Chain.PendingSendPackets) < 1 {
		return nil, errors.New("no packet to relay")
	}

	// grab the first pending packet
	//packet := channeltypes.Packet{}
	packet := src.Chain.PendingSendPackets[0]
	src.Chain.PendingSendPackets = src.Chain.PendingSendPackets[1:]

	if err := path.EndpointB.UpdateClient(); err != nil {
		return nil, err
	}

	res, err := path.EndpointB.RecvPacketWithResult(packet)
	if err != nil {
		return nil, err
	}

	ack, err = ibctesting.ParseAckFromEvents(res.GetEvents())
	if err != nil {
		return nil, err
	}

	if err = path.EndpointA.AcknowledgePacket(packet, ack); err != nil {
		return nil, err
	}

	for _, event := range res.GetEvents() {
		fmt.Println("event_type:", event.Type)
		for _, attr := range event.Attributes {
			fmt.Println(" - key:", string(attr.Key))
			fmt.Println("   value:", string(attr.Value))
		}
	}

	fmt.Println("ack:", string(ack))

	return ack, err
}

func Test(t *testing.T) {
	suite.Run(t, new(testSuite))
}
