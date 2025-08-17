import { initializeClient } from "./client";
import { Networks, SupportedNetworks } from "./config";
import { v4 as uuidv4 } from "uuid";

export const instantiateCW20 = async (targetNetwork: SupportedNetworks): Promise<string> => {
    let config = Networks[targetNetwork];
    if (!config.codeId.cw20) {
        console.log("no cw20 code id");
        return "";
    }
    let [client, userAddress] = await initializeClient(targetNetwork);
    const msg = {
        decimals: 6,
        initial_balances: [],
        name: config.lstCoinDenom,
        symbol: config.lstCoinSymbol,
        mint: {
            minter: userAddress
        }
    };

    try {
        const instantiateOptions = {
            memo: "Instantiating a new contract",
            funds: [],
            admin: userAddress,
        };

        const instantiateResult = await client?.instantiate(
            userAddress,
            config.codeId.cw20,
            msg,
            "cw20",
            "auto",
            instantiateOptions
        );
        //console.log(instantiateResult?.contractAddress);
        return instantiateResult?.contractAddress;

    } catch (err) {
        console.log(err);
    }

    return "";
};



export const instantiateLST = async (targetNetwork: SupportedNetworks, cw20Address: string): Promise<string> => {
    let config = Networks[targetNetwork];
    if (!config.codeId.lst) {
        console.log("no lst code id");
        return "";
    }
    let [client, userAddress] = await initializeClient(targetNetwork);

    try {
        const msg = {
            underlying_coin_denom: config.coinDenom,
            underlying_coin_denom_symbol: config.coinDenom,
            liquidstaking_denom: config.lstCoinDenom,
            liquidstaking_denom_symbol: config.lstCoinSymbol,
            validators: [
                { weight: 1, address: config.validator }
            ],
            ucs03_relay_contract: config.ucs03RelayContract,
            fee_receiver: config.revenueReceiver,
            unbonding_time: 600,
            reward_code_id: config.codeId.reward,
            fee_rate: "0.1",
            cw20_address: cw20Address,
            salt: uuidv4(),
            quote_tokens: [],
            batch_period: 28800,
            min_bond: "10000",
            min_unbond: "10000",
            batch_limit: 50,
            transfer_handler: "union1vnglhewf3w66cquy6hr7urjv3589srheampz42",
            transfer_fee: "0",
            zkgm_token_minter: "union1t5awl707x54k6yyx7qfkuqp890dss2pqgwxh07cu44x5lrlvt4rs8hqmk0",
            hub_channel_id: 19,
            hub_quote_token: "0xba5eD44733953d79717F6269357C77718C8Ba5ed",
            hub_contract: "0x15Ee7c367F4232241028c36E720803100757c6e9", // Replace with actual hub
        };



        const instantiateOptions = {
            memo: "Instantiating a new contract",
            funds: [],
            admin: userAddress,
        };

        const instantiateResult = await client?.instantiate(
            userAddress,
            config.codeId.lst,
            msg,
            "lst",
            "auto",
            instantiateOptions
        );
        //console.log(instantiateResult?.contractAddress);
        return instantiateResult?.contractAddress;

    } catch (err) {
        console.log(err);
    }

    return "";
}


const hexes = /*#__PURE__*/ Array.from({ length: 256 }, (_v, i) =>
    i.toString(16).padStart(2, '0'),
)

export function toHex(value: Uint8Array): string {
    let string = ''
    for (let i = 0; i < value.length; i++) {
        string += hexes[value[i]]
    }
    const hex = `0x${string}`;
    return hex;
}


export function getSalt() {
    const rawSalt = new Uint8Array(32)
    crypto.getRandomValues(rawSalt)
    const salt = toHex(rawSalt);
    return salt
}



export const transfer = async (targetNetwork: SupportedNetworks, contract: string, amount: string, hub_channel_id: number, recipient: string): Promise<string> => {
    let config = Networks[targetNetwork];
    if (!config.codeId.lst) {
        console.log("no lst code id");
        return "";
    }
    let [client, userAddress] = await initializeClient(targetNetwork);

    try {
        const msg = {
            transfer_and_call: {
                contract,
                amount,
                channel_id: hub_channel_id,
                recipient,
                salt: getSalt(),
            }
        };


        let funds = [
            {
                denom: "au",
                amount: amount
            }
        ];

        const res = await client?.execute(
            userAddress,
            contract,
            msg,
            "auto",
            "execute transfer", funds
        );
        return res?.transactionHash;

    } catch (err) {
        console.log(err);
    }

    return "";
}