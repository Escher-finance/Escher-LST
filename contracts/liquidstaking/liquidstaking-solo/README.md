Uploadcosmwasm

babylond tx wasm store artifacts/liquidstaking.wasm --from isak --gas auto --fees 13500ubbn --gas-adjustment 1.4


babylond tx wasm migrate bbn1qmayg959zunza00s040ppqesf7qnvusys3r2m9vw35ry28x9sncq84jphy 236 '{}' --from isak --gas auto --fees 5000ubbn --gas-adjustment 1.4


babylond tx wasm migrate bbn1ug4tume0pw6d4u7r6rhae6cp3udyrv7cr0angx8qegw7ur25sdxq4krcss 548 '{}' --from isak --gas auto --fees 5000ubbn --gas-adjustment 1.4







babylond tx wasm store artifacts/liquidstaking.wasm --from isak --gas auto --fees 13500ubbn --gas-adjustment 1.4 --generate-only > multisig/uploadv0.1.121.json