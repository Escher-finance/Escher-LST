.PHONY: deploy deploy-l1read

TESTNET_RPC_URL = $(shell grep TESTNET_RPC_URL .env | cut -d '=' -f2)
PRIVATE_KEY = $(shell grep PRIVATE_KEY .env | cut -d '=' -f2)
TESTNET_NETWORK_ARGS = --rpc-url $(TESTNET_RPC_URL) --private-key $(PRIVATE_KEY) --broadcast
READ_TESTNET_NETWORK_ARGS = --fork-url $(TESTNET_RPC_URL)

USDC_TESTNET = 0x2b3370ee501b4a559b57d449569354196457d8ab
VALIDATOR = 0xeb107f1b6ed0e4498898e351c3dfdfff9a1c1fdc

VALIDATOR_MANAGER=$(shell grep VALIDATOR_MANAGER .env | cut -d '=' -f2)
DELEGATION_MANAGER=$(shell grep VALIDATOR_MANAGER .env | cut -d '=' -f2)
LST=$(shell grep LST_ADDRESS .env | cut -d '=' -f2)
LST_MANAGER=$(shell grep LST_MANAGER .env | cut -d '=' -f2)

deployLst:
	@forge script script/DeployLiquidStakingManager.s.sol:DeployLiquidStakingManager $(TESTNET_NETWORK_ARGS) -vvvv --skip-simulation

deployH:
	@forge script script/DeployHValidatorDelegationManager.s.sol:DeployHyperliquidValidatorDelegationManager $(TESTNET_NETWORK_ARGS) -vvvv --skip-simulation

deployS:
	@forge script script/DeploySValidatorDelegationManager.s.sol:DeployStablechainValidatorDelegationManager $(TESTNET_NETWORK_ARGS) -vvvv --skip-simulation

l1read:
	@echo "Checking coreUserExists for user 0x15Ee7c367F4232241028c36E720803100757c6e9..."
	@cast call 0xb0FBCF71E600383C1298413BFDEFc4A32240B033 \
		"coreUserExists(address)(bool)" \
		0x15Ee7c367F4232241028c36E720803100757c6e9 \
		--rpc-url $(TESTNET_RPC_URL)

verify:
	@forge verify-contract $(LST_MANAGER) solidity/evm-lst/src/LiquidStakingManager.sol:LiquidStakingManager  \
	--chain-id 998 \
	--compiler-version v0.8.30 \
    --verifier sourcify \
    --verifier-url https://sourcify.parsec.finance/verify --rpc-url $(TESTNET_RPC_URL)

verifyV:
	@forge verify-contract $(VALIDATOR_MANAGER) solidity/evm-lst/src/ValidatorSetManager.sol:ValidatorSetManager  --chain-id 998 \
	--compiler-version v0.8.28 \
    --verifier sourcify \
    --verifier-url https://sourcify.parsec.finance/verify --rpc-url $(TESTNET_RPC_URL)

verifyD:
	@forge verify-contract $(DELEGATION_MANAGER) solidity/evm-lst/src/contracts/HyperliquidDelegationManager.sol:HyperliquidDelegationManager  --chain-id 998 \
	--compiler-version v0.8.28 \
    --verifier sourcify \
    --verifier-url https://sourcify.parsec.finance/verify --rpc-url $(TESTNET_RPC_URL)

balance:
	cast balance 0xbd7D62137de1a11EaA1860E6B57D46e3FCEF9f5B --rpc-url $(TESTNET_RPC_URL)

my_balance:
	cast balance 0x15Ee7c367F4232241028c36E720803100757c6e9 --rpc-url $(TESTNET_RPC_URL)

debug:
	cast run --debug --rpc-url $(TESTNET_RPC_URL) 0x88e3fb8d28220e95f17405ee70a4790e8517c0a04e2c60a31e409f5b3a792b17

delegate:
	@echo "Delegate ..."
	@cast send $(DELEGATION_MANAGER) \
		"delegate(uint256)" \
		1000000000000 \
		--value 1000000000000 \
		--rpc-url $(TESTNET_RPC_URL)  \
		--private-key $(PRIVATE_KEY) -vvvv

undelegate:
	@echo "Undelegate ..."
	@cast send $(DELEGATION_MANAGER) \
		"undelegate(uint256)" \
		100 \
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

updateValidators:
	@echo "update validators ..."
	@cast send $(DELEGATION_MANAGER) \
		"updateValidators(address[], uint64[])" \
		[0x056997a0a5da08dca9410945fb7aa8daba39d45d] \
		[1] \
		--rpc-url $(TESTNET_RPC_URL)  \
		--private-key $(PRIVATE_KEY) -vvvv

balance_gusdt:
	@echo "check balance ..."
	@cast call 0x0000000000000000000000000000000000001000 \
		"balanceOf(address)" \
		0x15Ee7c367F4232241028c36E720803100757c6e9 \
		--rpc-url $(TESTNET_RPC_URL)  \
		--private-key $(PRIVATE_KEY) -vvvv

balance_g:
	cast balance 0x15Ee7c367F4232241028c36E720803100757c6e9 --rpc-url $(TESTNET_RPC_URL)


validators:
	cast call 0x0000000000000000000000000000000000000800 \
    "validators(string,(bytes,uint64,uint64,bool,bool))" \
    "BOND_STATUS_BONDED" \
    "(0x,0,100,false,false)" \
    --rpc-url  $(TESTNET_RPC_URL)

# delegate:
# 	cast send 0x0000000000000000000000000000000000000800 \
#     "delegate(address,address,uint256)" \
#     0x15Ee7c367F4232241028c36E720803100757c6e9 0x1e81A5aCE3cF0ad903A5E78842ac7604e7b2Ad7C 1000 \
# 	--value 1000 \
#     --rpc-url https://rpc.testnet.stable.xyz  \
# 	--private-key $(PRIVATE_KEY) -vvvv

balance_stable:
	@echo "check balance ..."
	@cast call 0x0000000000000000000000000000000000001003  \
		"balanceOf(address)" \
		0x15Ee7c367F4232241028c36E720803100757c6e9 \
		--rpc-url $(TESTNET_RPC_URL)  \

receipt:
	@echo "check receipt ..."
	@cast receipt 0xaaf0ff1d9dbec3392d063f46e080a762eb31a909c0dfa9e78d1be79f3aa24256 \
	--rpc-url $(TESTNET_RPC_URL)
