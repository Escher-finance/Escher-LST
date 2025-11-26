.PHONY: deploy deploy-l1read

TESTNET_RPC_URL = $(shell grep TESTNET_RPC_URL .env | cut -d '=' -f2)
PRIVATE_KEY = $(shell grep PRIVATE_KEY .env | cut -d '=' -f2)
TESTNET_NETWORK_ARGS = --rpc-url $(TESTNET_RPC_URL) --private-key $(PRIVATE_KEY) --broadcast
READ_TESTNET_NETWORK_ARGS = --fork-url $(TESTNET_RPC_URL)

deploy:
	@forge script script/DeployDelegationManager.s.sol:DeployDelegationManager $(TESTNET_NETWORK_ARGS) -vvvv

l1read:
	@echo "Checking coreUserExists for user 0x15Ee7c367F4232241028c36E720803100757c6e9..."
	@cast call 0xb0FBCF71E600383C1298413BFDEFc4A32240B033 \
		"coreUserExists(address)(bool)" \
		0x15Ee7c367F4232241028c36E720803100757c6e9 \
		--rpc-url $(TESTNET_RPC_URL)

verify:
	@forge verify-contract 0xCAc44d624c08879ef96C59fff2f2187Bc014f415 src/DelegationManager.sol:DelegationManager  --chain-id 998 \
    --verifier sourcify \
    --verifier-url https://sourcify.parsec.finance/verify --rpc-url $(TESTNET_RPC_URL)

balance:
	cast balance 0x15Ee7c367F4232241028c36E720803100757c6e9 --rpc-url $(TESTNET_RPC_URL)

debug:
	cast run --debug --rpc-url $(TESTNET_RPC_URL) 0x88e3fb8d28220e95f17405ee70a4790e8517c0a04e2c60a31e409f5b3a792b17

delegate:
	@echo "Delegate ..."
	@cast send 0xCAc44d624c08879ef96C59fff2f2187Bc014f415 \
		"delegate()" \
		--value 1000000000000000 \
		--rpc-url $(TESTNET_RPC_URL)  \
		--private-key $(PRIVATE_KEY) -vvvv

sendcore:
	@echo "sendCore ..."
	@cast send 0xCAc44d624c08879ef96C59fff2f2187Bc014f415 \
		"sendCore()" \
		--value 100000000000000000 \
		--rpc-url $(TESTNET_RPC_URL)  \
		--private-key $(PRIVATE_KEY) -vvvv

deposit:
	@echo "deposit to staking account ..."
	@cast send 0xCAc44d624c08879ef96C59fff2f2187Bc014f415 \
		"depositStake(uint64)" \
		10000000 \
		--rpc-url $(TESTNET_RPC_URL)  \
		--private-key $(PRIVATE_KEY) -vvvv

stake:
	@echo "delegate to validator ..."
	@cast send 0xCAc44d624c08879ef96C59fff2f2187Bc014f415 \
		"stake(uint64)" \
		10000000 \
		--rpc-url $(TESTNET_RPC_URL)  \
		--private-key $(PRIVATE_KEY) -vvvv
