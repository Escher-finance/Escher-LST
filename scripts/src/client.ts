import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { DirectSecp256k1HdWallet, OfflineDirectSigner, OfflineSigner } from "@cosmjs/proto-signing";
import { GasPrice } from "@cosmjs/stargate";
const fs = require("fs");
import dotenv from 'dotenv';
dotenv.config()

import { SupportedNetworks, Networks } from "./config";

var MNEMONIC = process.env.MNEMONIC;

var client: SigningCosmWasmClient;
var userAddress: string;

export const getSignerFromMnemonic = async (): Promise<OfflineDirectSigner> => {
    if (!MNEMONIC) {
        throw "require mnemonic";
    }
    return DirectSecp256k1HdWallet.fromMnemonic(MNEMONIC, {
        prefix: "union",
    })
};


export const initializeClient = async (network: SupportedNetworks): Promise<[SigningCosmWasmClient, string]> => {
    let signer = await getSignerFromMnemonic();
    let accounts = await signer.getAccounts();
    userAddress = accounts[0].address;
    let rpc = Networks[network].rpc;
    if (!client) {
        client = await SigningCosmWasmClient.connectWithSigner(rpc, signer, {
            gasPrice: GasPrice.fromString("1au"),
        });
    }
    return [client, userAddress];
};



export const upload = async (wasmFilePath: string) => {
    console.log("user Address", userAddress);
    const wasm = fs.readFileSync(wasmFilePath);
    const uploadResult = await client.upload(userAddress, wasm, "auto", "");

    const codeId = uploadResult?.codeId;
    return codeId;
};


