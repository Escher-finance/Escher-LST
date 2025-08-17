import { initializeClient, upload } from "./client";
import { Networks, SupportedNetworks } from "./config";
import { instantiateCW20, instantiateLST, transfer } from "./instantiate";
import dotenv from 'dotenv';
dotenv.config()
let targetNetwork: SupportedNetworks = "uniontestnet";

var CW20_WASM_PATH = process.env.CW20_WASM_PATH;
var LST_WASM_PATH = process.env.LST_WASM_PATH;
var REWARD_WASM_PATH = process.env.REWARD_WASM_PATH;

console.log("CW20_WASM_PATH", CW20_WASM_PATH);
console.log("LST_WASM_PATH", LST_WASM_PATH);
console.log("REWARD_WASM_PATH", REWARD_WASM_PATH);

const uploadContracts = async (targetNetwork: SupportedNetworks) => {
    if (!CW20_WASM_PATH || !LST_WASM_PATH || !REWARD_WASM_PATH) {
        console.log("please setup the required env vars for cw20, lst, reward wasm path");
        return;
    }
    await initializeClient(targetNetwork);
    // let cw20CodeId = await upload(CW20_WASM_PATH);
    // console.log("cw20CodeId", cw20CodeId);
    let lstCodeId = await upload(LST_WASM_PATH);
    console.log("lstCodeId", lstCodeId);
    // let rewardCodeId = await upload(REWARD_WASM_PATH);
    // console.log("rewardCodeId", rewardCodeId);

    // Networks[targetNetwork].codeId.cw20 = cw20CodeId;
    // Networks[targetNetwork].codeId.lst = lstCodeId;
    // Networks[targetNetwork].codeId.reward = rewardCodeId;
    // console.log(JSON.stringify(Networks[targetNetwork].codeId));
}

const instantiate = async () => {
    console.log("instantiate");
    let cw20Address = await instantiateCW20(targetNetwork);
    console.log("cw20Address", cw20Address);
    let lstAddress = await instantiateLST(targetNetwork, cw20Address);
    console.log("lstAddress", lstAddress);
    await checkParams(lstAddress);
}

const runUpload = async () => {
    console.log("Setting up contracts for network:", targetNetwork);
    await uploadContracts(targetNetwork);
}

const checkParams = async (lstContract: string) => {
    let [client, _] = await initializeClient(targetNetwork);
    let params = await client.queryContractSmart(lstContract, { parameters: {} });
    console.log("params", JSON.stringify(params));
}

const runTransfer = async (receiver: string) => {
    console.log("Setting up contracts for network:", targetNetwork);
    let lst = "union17auke0f2l9uejk6wfkwvhlap3fxdqcdl8jkcmg6cmc53cnqrhekq7928m0";
    let channel_id = 19;
    let amount = "2000000";
    let hash = await transfer(targetNetwork, lst, amount, channel_id, receiver);
    console.log("Transfer hash:", hash);
}

const runMigrate = async () => {
    let lstContract = "union17auke0f2l9uejk6wfkwvhlap3fxdqcdl8jkcmg6cmc53cnqrhekq7928m0";
    let [client, userAddress] = await initializeClient(targetNetwork);
    let res = await client.migrate(userAddress, lstContract, 159, {}, "auto");
    console.log("Migrate hash:", res.transactionHash);
}


// runUpload();
// instantiate();
runTransfer("0x1285a2214319Eff512C5035933ac44E573738772");
// runMigrate();