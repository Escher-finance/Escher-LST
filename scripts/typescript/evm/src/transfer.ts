import { ethers } from "ethers";
import { getSalt, getTimeoutInNanoseconds7DaysFromNow } from "./utils.js";
import { Address, erc20Abi } from "viem";
import { ucs03abi } from "@unionlabs/sdk/evm/abi/ucs03";
import { encodeTokenOrderV2, tokenOrderV2Unescrow } from "./protocolV2.js";

const BABY_ON_BABYLON_HEX = "0x7562626e"; //ubbn
const HOLESKY_ERC20_BABY = "0x77b99a27a5fed3bc8fb3e2f1063181f82ec48637"; // ERC20 of BABY on holesky
const SEPOLIA_ERC20_BABY = "0xbd030914ab8d7ab1bd626f09e47c7cc2881550a3";

const ETHEREUM_ERC20_BABY = "0xe53dcec07d16d88e386ae0710e86d9a400f83c31";
const ETHEREUM_ERC20_EBABY = "0x70df20655b3e294facb436383435754dbee3cd70";
const UCS03_ADDRESS = "0x5fbe74a283f7954f10aa04c2edf55578811aeb03";

export const transferBabyFromEthToBabylon = async (
  signer: ethers.Wallet,
  amount: bigint,
  receiver: string,
  channelId: bigint
) => {
  let sender = await signer.getAddress();

  const baseToken = ETHEREUM_ERC20_BABY; //erc20 baby on Eth

  let quoteToken: Address = BABY_ON_BABYLON_HEX; //Denom name in babylon (ubbn) in hex
  let salt = getSalt();
  console.log(salt);

  // //approve ucs03 contract to transfer first
  const erc20Contract = new ethers.Contract(baseToken, erc20Abi, signer);
  const resp = await erc20Contract.approve(UCS03_ADDRESS, amount);
  console.log(resp);

  let txReceipt = await resp.wait();
  console.log(txReceipt);

  let tokenOrder = tokenOrderV2Unescrow(
    sender.toLowerCase(),
    receiver,
    baseToken,
    amount,
    quoteToken,
    amount
  );

  const ucs03Contract = new ethers.Contract(UCS03_ADDRESS, ucs03abi, signer);

  const transferRes = await ucs03Contract.send(
    channelId,
    0,
    getTimeoutInNanoseconds7DaysFromNow(),
    salt,
    {
      opcode: tokenOrder.opcode,
      version: tokenOrder.version,
      operand: encodeTokenOrderV2(tokenOrder),
    },
    { gasLimit: 500000 } // Adjust gas limit as needed
  );

  console.log(transferRes);

  let transferRecepit = await transferRes.wait();
  console.log(transferRecepit);
};
