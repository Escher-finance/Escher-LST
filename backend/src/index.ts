import { getClient, getSignerFromMnemonic } from "./client";
import dotenv from 'dotenv';
dotenv.config()

var RPC_URL = process.env.RPC_URL;
var MNEMONIC = process.env.MNEMONIC;
var LST_CONTRACT_ADDRESS = process.env.LST_CONTRACT_ADDRESS;

export async function processUnbonding(id: number) {
    if (!RPC_URL || !MNEMONIC || !LST_CONTRACT_ADDRESS) {

        return;
    }

    let signer = await getSignerFromMnemonic(MNEMONIC);
    let client = await getClient(signer, RPC_URL);
    let accounts = await signer.getAccounts();
    const userAddress = accounts[0].address;
    console.log("Process Unbonding: " + id.toString() + " from: " + JSON.stringify(accounts));

    let msg = {
        process_unbonding: {
            id
        }
    };

    try {
        const res = await client.execute(userAddress, LST_CONTRACT_ADDRESS, msg, "auto");
        console.log(res.transactionHash);
    } catch (e: any) {
        console.log("Error ", e);
    }
}

export async function processRewards() {
    if (!RPC_URL || !MNEMONIC || !LST_CONTRACT_ADDRESS) {

        return;
    }

    let signer = await getSignerFromMnemonic(MNEMONIC);
    let client = await getClient(signer, RPC_URL);
    let accounts = await signer.getAccounts();
    const userAddress = accounts[0].address;
    console.log("Process Rewards ");

    let msg = {
        process_rewards: {}
    };

    try {
        const res = await client.execute(userAddress, LST_CONTRACT_ADDRESS, msg, "auto");
        console.log(res.transactionHash);
    } catch (e: any) {
        console.log("Error ", e);
    }
}

export async function checkUnbonding() {
    console.log("checkUnbonding, RPC_URL:" + RPC_URL);
    if (!RPC_URL || !MNEMONIC || !LST_CONTRACT_ADDRESS) {
        console.log("RPC_URL or MNEMONIC or LST_CONTRACT_ADDRESS is empty");
        return;
    }
    console.log("start checkUnbonding");
    let signer = await getSignerFromMnemonic(MNEMONIC);
    let client = await getClient(signer, RPC_URL);

    try {
        const msg: any = {
            unbond_record: {
                released: false
            }
        };

        const records = await client.queryContractSmart(
            LST_CONTRACT_ADDRESS,
            msg
        );

        let currentTimestamp = new Date().getTime();
        for (var i = 0; i < records.length; i++) {
            let unbondingTimestamp = Math.ceil(records[i].completion / 1000000);
            if (currentTimestamp > unbondingTimestamp) {
                console.log(JSON.stringify(records[i]));
                processUnbonding(records[i].id);
            }
        };

    } catch (e: any) {
        console.log("Error ", e);
    }
}

const args = process.argv;

const act = process.env.npm_config_name ? process.env.npm_config_name : args[2]

console.log("Action: " + act);
if (act == "process_unbonding") {
    checkUnbonding()
} else if (act == "process_rewards") {
    processRewards();
} else {
    console.log(">>> do nothing");
}