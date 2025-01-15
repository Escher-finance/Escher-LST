import Networks, { ChainConfig } from "../../config/networks.config";
import { LocalStorage } from "../lib/localstorage";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { GasPrice } from "@cosmjs/stargate";
import type { Address } from "viem"
import { type OfflineSigner } from "@unionlabs/client";

export async function initializeKeplr(
  network: ChainConfig | null,
  setClient: (client: SigningCosmWasmClient) => void,
  setUserAddress: (addr: string) => void,
  setAuthenticated: (val: boolean) => void
) {
  try {
    await (window as any).keplr.experimentalSuggestChain(network);
    (window as any).keplr.defaultOptions = {
      sign: { preferNoSetFee: true },
    };
    (window as any).keplr.enable(network?.chainId);

    if (network) setClientNomos(network, "keplr", setClient, setUserAddress, setAuthenticated);
  } catch (e) {
    alert("Failed to suggest the chain:" + e);
  }
}

export async function initializeLeap(
  network: ChainConfig | null,
  setClient: (client: SigningCosmWasmClient) => void,
  setUserAddress: (addr: string) => void,
  setAuthenticated: (val: boolean) => void
) {
  try {
    await (window as any).leap.experimentalSuggestChain(network);
    (window as any).leap.defaultOptions = {
      sign: { preferNoSetFee: true },
    };
    (window as any).leap.enable(network?.chainId);

    if (network) setClientNomos(network, "leap", setClient, setUserAddress, setAuthenticated);
  } catch {
    alert("Failed to suggest the chain");
  }
}

export async function initializeCosmos(
  network: ChainConfig | null,
  setClient: (client: SigningCosmWasmClient) => void,
  setUserAddress: (addr: string) => void,
  setAuthenticated: (val: boolean) => void
) {
  try {
    if (network)
      setClientNomos(network, "cosmostation", setClient, setUserAddress, setAuthenticated);
  } catch {
    alert("Failed to suggest the chain");
  }
}

export async function setClientNomos(
  network: ChainConfig,
  selectedWallet: string | null,
  setClient: (client: SigningCosmWasmClient) => void,
  setUserAddress: (addr: string) => void,
  setAuthenticated: (val: boolean) => void
) {
  if (selectedWallet == null) selectedWallet = LocalStorage.getWallet();
  console.log(JSON.stringify(selectedWallet));

  let offlineSigner = undefined;

  switch (selectedWallet) {
    case "keplr":
      (window as any).wallet = (window as any).keplr;
      //console.log("NETWORK", network);
      offlineSigner = await getCosmosOfflineSigner({ chainId: network?.chainId, connectedWallet: "keplr" });
      break;
    case "leap":
      (window as any).wallet = (window as any).leap;
      offlineSigner = await getCosmosOfflineSigner({ chainId: network?.chainId, connectedWallet: "leap" });
      break;
  }

  if (offlineSigner) {
    const cosmosClient = await SigningCosmWasmClient.connectWithSigner(network.rpc, offlineSigner, {
      gasPrice: GasPrice.fromString(network.gasPrice),
    });
    //console.log("setClientNomos");
    //console.log(cosmosClient);
    setClient(cosmosClient);

    const accounts = await offlineSigner.getAccounts();

    if (accounts) {
      setUserAddress(accounts[0].address);
      LocalStorage.setUserAddress(accounts[0].address);

      setAuthenticated(true);
      LocalStorage.setAuthenticated(true);
      LocalStorage.setWallet(selectedWallet);

      console.log("setWalletAndAddress", {
        offlineSigner: offlineSigner,
        cosmosClient: cosmosClient,
      });
    }
  }
}


export type UserAddressCosmos = {
  canonical: string
  normalized: string
  bytes: Uint8Array
  normalized_prefixed: Address
}


export const cosmosWalletsInformation = [
  {
    id: "leap",
    name: "leap",
    icon: "/images/icons/leap.svg",
    /**
     * reference links:
     * - leap deep link generator: https://developers.leapwallet.io/deeplink-generator
     * - qr code: https://git-union69.web.val.run/app.union.build?svg=union.build/logo.svg&url=leapcosmoswallet.page.link/M3BmzUK5RRPsNyBe9?d=1
     */
    deepLink: "https://leapcosmoswallet.page.link/rXtQWTw1fSRuQCeZ8?d=1",
    download: "https://leapwallet.io/download"
  },
  /**
   * reference links:
   * - keplr link generator: https://chainapsis.notion.site/How-to-use-Deep-Link-on-Keplr-mobile-5593b89de65142278a6a7573c97ad0d4
   * - qr code: https://git-union69.web.val.run/app.union.build?svg=union.build/logo.svg&url=leapcosmoswallet.page.link/M3BmzUK5RRPsNyBe9?d=1
   */
  {
    id: "keplr",
    name: "keplr",
    icon: "/images/icons/keplr.svg",
    deepLink:
      "https://deeplink.keplr.app?url=app.union.build#Intent;package=com.chainapsis.keplr;scheme=keplrwallet;end;",
    download: "https://keplr.app/download"
  }
] as const

export type CosmosWalletId = (typeof cosmosWalletsInformation)[number]["id"]

export const getCosmosOfflineSigner = ({
  chainId,
  connectedWallet
}: {
  chainId: string
  connectedWallet: CosmosWalletId
}): Promise<OfflineSigner> =>
  // @ts-expect-error
  window[connectedWallet]?.getOfflineSignerAuto(chainId, { disableBalanceCheck: false })
