import Networks, { ChainConfig } from "../../config/networks.config";
import { LocalStorage } from "../lib/localstorage";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { GasPrice } from "@cosmjs/stargate";
import { getOfflineSigner } from "@cosmostation/cosmos-client";

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
      console.log("NETWORK", network);
      offlineSigner = (window as any).wallet.getOfflineSigner(network.chainId);
      break;
    case "leap":
      (window as any).wallet = (window as any).leap;
      offlineSigner = (window as any).wallet.getOfflineSigner(network.chainId);
      break;
    case "cosmostation":
      offlineSigner = await getOfflineSigner(network.chainId);
      break;
  }

  const cosmosClient = await SigningCosmWasmClient.connectWithSigner(network.rpc, offlineSigner, {
    gasPrice: GasPrice.fromString(network.gasPrice),
  });
  console.log("setClientNomos");
  console.log(cosmosClient);
  setClient(cosmosClient);

  const accounts = await offlineSigner.getAccounts();

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
