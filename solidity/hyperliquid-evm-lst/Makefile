.PHONY: deploy deploy-l1read

TESTNET_RPC_URL = $(shell grep TESTNET_RPC_URL .env | cut -d '=' -f2)
PRIVATE_KEY = $(shell grep PRIVATE_KEY .env | cut -d '=' -f2)
TESTNET_NETWORK_ARGS = --rpc-url $(TESTNET_RPC_URL) --private-key $(PRIVATE_KEY) --broadcast
READ_TESTNET_NETWORK_ARGS = --fork-url $(TESTNET_RPC_URL)

DELEGATION_MANAGER = 0xf015054D7B7Fa81793f0ac8E4B4dC8bb71551f74
USDC_TESTNET = 0x2b3370ee501b4a559b57d449569354196457d8ab
VALIDATOR = 0xeb107f1b6ed0e4498898e351c3dfdfff9a1c1fdc

deploy:
	@forge script script/DeployDelegationManager.s.sol:DeployDelegationManager $(TESTNET_NETWORK_ARGS) -vvvv --skip-simulation

l1read:
	@echo "Checking coreUserExists for user 0x15Ee7c367F4232241028c36E720803100757c6e9..."
	@cast call 0xb0FBCF71E600383C1298413BFDEFc4A32240B033 \
		"coreUserExists(address)(bool)" \
		0x15Ee7c367F4232241028c36E720803100757c6e9 \
		--rpc-url $(TESTNET_RPC_URL)

verify:
	@forge verify-contract $(DELEGATION_MANAGER) src/DelegationManager.sol:DelegationManager  --chain-id 998 \
    --verifier sourcify \
    --verifier-url https://sourcify.parsec.finance/verify --rpc-url $(TESTNET_RPC_URL)

balance:
	cast balance 0x15Ee7c367F4232241028c36E720803100757c6e9 --rpc-url $(TESTNET_RPC_URL)

debug:
	cast run --debug --rpc-url $(TESTNET_RPC_URL) 0x88e3fb8d28220e95f17405ee70a4790e8517c0a04e2c60a31e409f5b3a792b17

delegate:
	@echo "Delegate ..."
	@cast send $(DELEGATION_MANAGER) \
		"delegate()" \
		--value 1000000000000000 \
		--rpc-url $(TESTNET_RPC_URL)  \
		--private-key $(PRIVATE_KEY) -vvvv

sendcore:
	@echo "sendCore ..."
	@cast send $(DELEGATION_MANAGER) \
		"sendCore()" \
		--value 1000000000000000 \
		--rpc-url $(TESTNET_RPC_URL)  \
		--private-key $(PRIVATE_KEY) -vvvv

deposit:
	@echo "deposit to staking account ..."
	@cast send $(DELEGATION_MANAGER) \
		"depositStake(uint64)" \
		100000 \
		--rpc-url $(TESTNET_RPC_URL)  \
		--private-key $(PRIVATE_KEY) -vvvv

stake:
	@echo "delegate to validator ..."
	@cast send $(DELEGATION_MANAGER) \
		"stake(uint64)" \
		100000 \
		--rpc-url $(TESTNET_RPC_URL)  \
		--private-key $(PRIVATE_KEY) -vvvv

add_validator:
	@echo "add validator ..."
	@cast send $(DELEGATION_MANAGER) \
		"add_validator(address,uint256)" \
		0xeb107f1b6ed0e4498898e351c3dfdfff9a1c1fdc \
		1 \
		--rpc-url $(TESTNET_RPC_URL)  \
		--private-key $(PRIVATE_KEY) -vvvv
