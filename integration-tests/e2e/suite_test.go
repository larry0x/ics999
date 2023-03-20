package e2e_test

import (
	"encoding/json"
	"testing"

	"github.com/stretchr/testify/require"
	"github.com/stretchr/testify/suite"

	wasmibctesting "github.com/CosmWasm/wasmd/x/wasm/ibctesting"
	sdk "github.com/cosmos/cosmos-sdk/types"
	ibctesting "github.com/cosmos/ibc-go/v6/testing"

	"ics999/types"
)

type testSuite struct {
	suite.Suite

	coordinator *wasmibctesting.Coordinator

	chainA *testChain
	chainB *testChain
}

func (suite *testSuite) SetupTest() {
	suite.coordinator = wasmibctesting.NewCoordinator(suite.T(), 2)

	suite.chainA = setupChain(suite.T(), suite.coordinator.GetChain(wasmibctesting.GetChainID(0)))
	suite.chainB = setupChain(suite.T(), suite.coordinator.GetChain(wasmibctesting.GetChainID(1)))

	setupConnection(suite.coordinator, suite.chainA, suite.chainB)
}

type testChain struct {
	*wasmibctesting.TestChain

	core sdk.AccAddress // address of one-core contract
}

func setupChain(t *testing.T, chain *wasmibctesting.TestChain) *testChain {
	// store one-core contract code
	coreStoreRes := chain.StoreCodeFile("../../artifacts/one_core-aarch64.wasm")
	require.Equal(t, uint64(1), coreStoreRes.CodeID)

	// store one-account contract code
	accountStoreRes := chain.StoreCodeFile("../../artifacts/one_account-aarch64.wasm")
	require.Equal(t, uint64(2), accountStoreRes.CodeID)

	// instantiate one-core contract
	instantiateMsg, err := json.Marshal(&types.CoreInstantiateMsg{
		AccountCodeID:  accountStoreRes.CodeID,
		TransferCodeID: uint64(0), // FIXME: placeholder
	})
	require.NoError(t, err)

	core := chain.InstantiateContract(coreStoreRes.CodeID, instantiateMsg)

	return &testChain{
		TestChain: chain,
		core:      core,
	}
}

func setupConnection(coordinator *wasmibctesting.Coordinator, chainA, chainB *testChain) {
	path := wasmibctesting.NewPath(chainA.TestChain, chainB.TestChain)
	path.EndpointA.ChannelConfig = &ibctesting.ChannelConfig{
		PortID:  chainA.ContractInfo(chainA.core).IBCPortID,
		Order:   types.Order,
		Version: types.Version,
	}
	path.EndpointB.ChannelConfig = &ibctesting.ChannelConfig{
		PortID:  chainB.ContractInfo(chainB.core).IBCPortID,
		Order:   types.Order,
		Version: types.Version,
	}

	coordinator.SetupConnections(path)
	coordinator.CreateChannels(path)
}

func Test(t *testing.T) {
	suite.Run(t, new(testSuite))
}
