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
            validators: [
                { weight: 1, address: "cosmosvaloper1h492ust5a9qzhh4zhhhlyva9v8ftn5sz99k4yp" }
            ],
            liquidstaking_denom: config.lstCoinDenom,
            ucs01_channel: config.ucs01Channel,
            ucs01_relay_contract: config.ucs01RelayContract,
            fee_rate: config.feeRate,
            revenue_receiver: config.revenueReceiver,
            unbonding_time: 10,
            cw20_address: cw20Address,
            reward_code_id: config.codeId.reward,
            coin_denom: config.coinDenom,
            salt: uuidv4(),
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
