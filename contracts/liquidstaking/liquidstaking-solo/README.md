Uploadcosmwasm

babylond tx wasm store artifacts/liquidstaking.wasm --from isak --gas auto --fees 13500ubbn --gas-adjustment 1.4

babylond tx wasm migrate bbn1qmayg959zunza00s040ppqesf7qnvusys3r2m9vw35ry28x9sncq84jphy 236 '{}' --from isak --gas auto --fees 5000ubbn --gas-adjustment 1.4

babylond tx wasm migrate bbn1ug4tume0pw6d4u7r6rhae6cp3udyrv7cr0angx8qegw7ur25sdxq4krcss 575 '{}' --from isak --gas auto --fees 5000ubbn --gas-adjustment 1.4

babylond tx wasm store artifacts/liquidstaking.wasm --from isak --gas auto --fees 13500ubbn --gas-adjustment 1.4 --generate-only > multisig/uploadv0.1.121.json


UPDATE quote token

'{"update_quote_token":{"channel_id":3, "quote_token": { "channel_id" : 3, "quote_token":"0xe53dcec07d16d88e386ae0710e86d9a400f83c31", "lst_quote_token": "0x62626e316c6364736d6a64736139676c703638736771756670786776326d6e786175767663766b776a777072616a356472347561733032717a7030686664"}}}'


babylond --from ucs03 tx authz grant bbn1m7zr5jw4k9z22r9ajggf4ucalwy7uxvu9gkw6tnsmv42lvjpkwasagek5g generic --msg-type="/cosmwasm.wasm.v1.MsgExecuteContract" --fees 400ubbn