

import { ethers } from "ethers";
import { bondFromSepoliaToBabylon, bondFromHoleskyToUnion, bondFromHoleskyToBabylon, unbondFromSepoliaToBabylon, unbondFromHoleskyToBabylon } from "./staking.js";
import { transferBabyFromEthToBabylon } from "./transfer.js";
import { predictQuoteToken } from "./protocolV1.js";
import { toHex } from "viem";
import { getAddressFromEvm, predictWrappedTokenV2 } from "./protocolV2.js";
import { sign } from "crypto";
import { ChannelId } from "@unionlabs/sdk/schema/channel";
import { Effect } from "effect";

const HOLESKY_TO_BABYLON_CHANNEL_ID = BigInt(3);
const SEPOLIA_TO_BABYLON_CHANNEL_ID = BigInt(7);

const HOLESKY_TO_BABYLON_DESTINATION_CHANNEL_ID = 2;
const SEPOLIA_TO_BABYLON_DESTINATION_CHANNEL_ID = 1;

const BABYLON_UCS03 = "bbn1336jj8ertl8h7rdvnz4dh5rqahd09cy0x43guhsxx6xyrztx292q77945h";

//const BABYLON_RECEIVER = "bbn1vnglhewf3w66cquy6hr7urjv3589srheqj3myz";
var HOLESKY_RPC_URL = "https://holesky.drpc.org";
var SEPOLIA_RPC_URL = "https://1rpc.io/sepolia";

const bytecode_base_checksum = "0xec827349ed4c1fec5a9c3462ff7c979d4c40e7aa43b16ed34469d04ff835f2a1" as const;
const module_hash = "0x120970d812836f19888625587a4606a5ad23cef31c8684e601771552548fc6b9" as const;

const bondSepolia = async (signer: ethers.Wallet, channel_id: number, amount: bigint) => {
    const PROXY_ADDRESS = await getAddress(signer.address as `0x${string}`, ChannelId.make(SEPOLIA_TO_BABYLON_DESTINATION_CHANNEL_ID), BABYLON_UCS03, bytecode_base_checksum, module_hash);
    await bondFromSepoliaToBabylon(signer, amount, channel_id, PROXY_ADDRESS.address)
}

const unbondSepolia = async (signer: ethers.Wallet, channel_id: number, amount: bigint) => {
    const PROXY_ADDRESS = await getAddress(signer.address as `0x${string}`, ChannelId.make(SEPOLIA_TO_BABYLON_DESTINATION_CHANNEL_ID), BABYLON_UCS03, bytecode_base_checksum, module_hash);
    console.log(PROXY_ADDRESS);
    await unbondFromSepoliaToBabylon(signer, amount, channel_id, PROXY_ADDRESS.address)
}

const unbondHolesky = async (signer: ethers.Wallet, channel_id: number, amount: bigint) => {
    const PROXY_ADDRESS = await getAddress(signer.address as `0x${string}`, ChannelId.make(HOLESKY_TO_BABYLON_DESTINATION_CHANNEL_ID), BABYLON_UCS03, bytecode_base_checksum, module_hash);
    console.log(PROXY_ADDRESS);
    //await unbondFromHoleskyToBabylon(signer, amount, channel_id, PROXY_ADDRESS.address)
}

const bondHolesky = async (signer: ethers.Wallet, channel_id: number, amount: bigint) => {
    const PROXY_ADDRESS = await getAddress(signer.address as `0x${string}`, ChannelId.make(HOLESKY_TO_BABYLON_DESTINATION_CHANNEL_ID), BABYLON_UCS03, bytecode_base_checksum, module_hash);
    await bondFromHoleskyToBabylon(signer, amount, channel_id, PROXY_ADDRESS.address);
}

// const transfer = async (signer: ethers.Wallet, amount: bigint, receiver: string) => {
//     await transferBabyFromEthToBabylon(signer, amount, receiver, HOLESKY_TO_BABYLON_CHANNEL_ID);
// }

const getAddress = async (sender: `0x${string}`,
    channel: ChannelId,
    ucs03: `${string}1${string}`,
    bytecode_base_checksum: `0x${string}`,
    module_hash: `0x${string}`) => {
    const receiver = await Effect.runPromise(getAddressFromEvm({
        path: BigInt(0),
        channel,
        sender,
        ucs03,
        bytecode_base_checksum,
        module_hash,
    })
    );
    return receiver;
}

var holeskyProvider = new ethers.JsonRpcProvider(HOLESKY_RPC_URL);
var sepoliaProvider = new ethers.JsonRpcProvider(SEPOLIA_RPC_URL);
var privateKey = process.env.PRIVATE_KEY;


if (privateKey) {

    let amount = 11000n;
    // let sepoliaSigner = new ethers.Wallet(privateKey, sepoliaProvider);
    // unbondSepolia(sepoliaSigner, Number(SEPOLIA_TO_BABYLON_CHANNEL_ID), amount);

    let signer = new ethers.Wallet(privateKey, holeskyProvider);
    unbondHolesky(signer, Number(HOLESKY_TO_BABYLON_CHANNEL_ID), amount);

    //bond(signer, Number(HOLESKY_TO_BABYLON_CHANNEL_ID), amount);

} else {
    console.log("no private key in env var");
}




// const HOLESKY_BABYLON_RECEIVER = "bbn1w9kn4mqhgmtafyr4t2a660y7y7wxqv8u5gau2lx55xhyamu23jlqxwa5kx"; // Cw account of 0x15Ee7c367F4232241028c36E720803100757c6e9 on babylon from holesky
// //const SEPOLIA_BABYLON_RECEIVER = "bbn14st4ptuu4w4rkttxtzmw5872h0ufulesrwjpcjnek3d9lfnseegsphj4vn";
// let amount = 10000n;
// transfer(signer, amount, HOLESKY_BABYLON_RECEIVER);

// predict quote token of Baby on sepolia
//predictQuoteToken(signer, 7n, "0x7562626e");

// //predict wrapped token of eBaby on sepolia
//predictQuoteToken(signer, 7n, toHex("bbn1cnx34p82zngq0uuaendsne0x4s5gsm7gpwk2es8zk8rz8tnj938qqyq8f9"));

// //predict wrapped token of eBaby on holesky
// predictQuoteToken(signer, 3n, toHex("bbn1cnx34p82zngq0uuaendsne0x4s5gsm7gpwk2es8zk8rz8tnj938qqyq8f9"));

// predict wrapped token v2 of eBaby on sepolia (channel id = 7 from sepolia to babylon)
// cw20 of ebaby on babylon address = bbn1cnx34p82zngq0uuaendsne0x4s5gsm7gpwk2es8zk8rz8tnj938qqyq8f9
//predictWrappedTokenV2(signer, 7n, toHex("bbn1cnx34p82zngq0uuaendsne0x4s5gsm7gpwk2es8zk8rz8tnj938qqyq8f9"));



// getAddress("0x15Ee7c367F4232241028c36E720803100757c6e9", ChannelId.make(3), "bbn1336jj8ertl8h7rdvnz4dh5rqahd09cy0x43guhsxx6xyrztx292q77945h", bytecode_base_checksum, module_hash);