import Networks, { ChainConfig } from "../../config/networks.config";
import { LocalStorage } from "../lib/localstorage";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { GasPrice } from "@cosmjs/stargate";
import { type OfflineSigner } from "@unionlabs/client"

export async function initializeKeplr(
  network: ChainConfig | null,
  setCient: (client: any) => void,
  setUserAddress: (addr: string) => void,
  setAuthenticated: (val: boolean) => void
) {
  try {
    await (window as any).keplr.experimentalSuggestChain(network);
    (window as any).keplr.defaultOptions = {
      sign: { preferNoSetFee: true },
    };
    (window as any).keplr.enable(network?.chainId);

    if (network) setClientNomos(network, "keplr", setCient, setUserAddress, setAuthenticated);
  } catch {
    alert("Failed to suggest the chain");
  }
}

export async function initializeLeap(
  network: ChainConfig | null,
  setCient: (client: any) => void,
  setUserAddress: (addr: string) => void,
  setAuthenticated: (val: boolean) => void
) {
  try {
    await (window as any).leap.experimentalSuggestChain(network);
    (window as any).leap.defaultOptions = {
      sign: { preferNoSetFee: true },
    };
    (window as any).leap.enable(network?.chainId);

    if (network) setClientNomos(network, "leap", setCient, setUserAddress, setAuthenticated);
  } catch {
    alert("Failed to suggest the chain");
  }
}

export async function initializeCosmos(
  network: ChainConfig | null,
  setCient: (client: any) => void,
  setUserAddress: (addr: string) => void,
  setAuthenticated: (val: boolean) => void
) {
  try {
    if (network)
      setClientNomos(network, "cosmostation", setCient, setUserAddress, setAuthenticated);
  } catch {
    alert("Failed to suggest the chain");
  }
}

export async function setClientNomos(
  network: ChainConfig,
  selectedWallet: string | null,
  setCient: (client: any) => void,
  setUserAddress: (addr: string) => void,
  setAuthenticated: (val: boolean) => void
) {
  if (selectedWallet == null) selectedWallet = LocalStorage.getWallet();
  console.log(JSON.stringify(selectedWallet));

  let offlineSigner: OfflineSigner | undefined = undefined;

  switch (selectedWallet) {
    case "keplr":
      (window as any).wallet = (window as any).keplr;
      console.log("NETWORK", network);
      offlineSigner = await (window as any).wallet.getOfflineSignerAuto(network.chainId, { disableBalanceCheck: false });
      break;
    case "leap":
      (window as any).wallet = (window as any).leap;
      offlineSigner = await (window as any).wallet.getOfflineSignerAuto(network.chainId, { disableBalanceCheck: false });
      break;
  }

  if (offlineSigner) {
    const cosmosClient = await SigningCosmWasmClient.connectWithSigner(network.rpc, offlineSigner, {
      gasPrice: GasPrice.fromString(network.gasPrice),
    });
    console.log("setClientNomos");
    setCient(cosmosClient);
    const accounts = await offlineSigner?.getAccounts();

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
