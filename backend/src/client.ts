import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { DirectSecp256k1HdWallet, OfflineDirectSigner, OfflineSigner } from "@cosmjs/proto-signing";
import { GasPrice } from "@cosmjs/stargate";


export const getSignerFromMnemonic = async (mnemonic: string): Promise<OfflineDirectSigner> => {
    if (!mnemonic) {
        throw "require mnemonic";
    }
    return DirectSecp256k1HdWallet.fromMnemonic(mnemonic, {
        prefix: "cosmos",
    })
};


export const getClient = async (signer: OfflineSigner, rpc: string): Promise<SigningCosmWasmClient> => {
    return await SigningCosmWasmClient.connectWithSigner(rpc, signer, {
        gasPrice: GasPrice.fromString("0.001stake"),
    });
};




