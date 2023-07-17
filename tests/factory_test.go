package e2e_test

import (
	"encoding/json"

	"github.com/stretchr/testify/require"

	wasmibctesting "github.com/CosmWasm/wasmd/x/wasm/ibctesting"
	wasmvmtypes "github.com/CosmWasm/wasmvm/types"

	"ics999/tests/types"
)

type invalidData struct {
	Foo string `json:"foo"`
	Bar string `json:"bar"`
}

func (suite *testSuite) TestRegisterCustomFactory() {
	correctData, err := json.Marshal(&types.FactoryData{
		CodeID:         suite.chainB.accountCodeID,
		InstantiateMsg: []byte("{}"),
	})
	require.NoError(suite.T(), err)

	incorrectData, err := json.Marshal(&invalidData{
		Foo: "fuzz",
		Bar: "buzz",
	})
	require.NoError(suite.T(), err)

	// instantiate factory contract on chainB, with a config such that only the
	// sender contract on chainA can register
	factoryStoreRes := suite.chainB.StoreCodeFile("../artifacts/mock_account_factory.wasm")
	factoryInstantiateMsg, err := json.Marshal(&types.FactoryConfig{
		OneCore: suite.chainB.coreAddr.String(),
		AllowedSrc: wasmvmtypes.IBCEndpoint{
			PortID:    suite.pathAB.EndpointB.ChannelConfig.PortID,
			ChannelID: suite.pathAB.EndpointB.ChannelID,
		},
		AllowedController: suite.chainA.senderAddr.String(),
	})
	require.NoError(suite.T(), err)
	factoryAddr := suite.chainB.InstantiateContract(factoryStoreRes.CodeID, factoryInstantiateMsg)

	for _, tc := range []struct {
		desc        string
		senderChain *testChain
		path        *wasmibctesting.Path
		data        []byte
		expSuccess  bool
	}{
		{
			desc:        "disallowed source",
			senderChain: suite.chainC,
			path:        reversePath(suite.pathBC),
			data:        correctData,
			expSuccess:  false,
		},
		{
			desc:        "empty instantiate data",
			senderChain: suite.chainA,
			path:        suite.pathAB,
			data:        nil,
			expSuccess:  false,
		},
		{
			desc:        "invalid instantiate data",
			senderChain: suite.chainA,
			path:        suite.pathAB,
			data:        incorrectData,
			expSuccess:  false,
		},
		{
			desc:        "allowed source and sender, correct instantiate data",
			senderChain: suite.chainA,
			path:        suite.pathAB,
			data:        correctData,
			expSuccess:  true,
		},
	} {
		_, ack, err := act(tc.senderChain, tc.path, []types.Action{
			{
				RegisterAccount: &types.RegisterAccountAction{
					CustomFactory: &types.RegisterAccountCustomFactory{
						Address: factoryAddr.String(),
						Data:    tc.data,
					},
				},
			},
		})
		require.NoError(suite.T(), err)

		if tc.expSuccess {
			requirePacketSuccess(suite.T(), ack)

			// check if an account has been registered, and its address matches that
			// returned in the packet ack
			accountAddr, err := queryAccount(
				suite.chainB,
				suite.pathAB.EndpointB.ChannelID,
				suite.chainA.senderAddr.String(),
			)
			require.NoError(suite.T(), err)
			require.Equal(suite.T(), ack.Success[0].RegisterAccount.Address, accountAddr.String())
		} else {
			requirePacketFailed(suite.T(), ack)
		}
	}
}
