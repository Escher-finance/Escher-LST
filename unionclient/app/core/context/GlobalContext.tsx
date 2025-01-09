"use client";

import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import Networks, { ChainConfig } from "../../../config/networks.config";
import { createContext, useState } from "react";

interface IGlobalContextProps {
  authenticated: boolean;
  setAuthenticated: (authenticated: boolean) => void;

  userAddress: string | null;
  setUserAddress: (userAddress: string | null) => void;

  network: ChainConfig;
  setNetwork: (network: ChainConfig) => void;

  client: SigningCosmWasmClient | undefined;
  setClient: (client: SigningCosmWasmClient) => void;

  icaAddress: string | null;
  setICAAddress: (icaAddress: string | null) => void;


  icaControllerAddress: string | null;
  setICAControllerAddress: (icaControllerAddress: string | null) => void;


  queryClient: any;
  setQueryClient: (client: any) => void;

}

interface Props {
  children: React.ReactNode;
}

export const GlobalContext = createContext<IGlobalContextProps>({
  authenticated: false,
  setAuthenticated: () => { },
  userAddress: null,
  setUserAddress: () => { },
  network: Networks["lst-network"],
  setNetwork: () => { },
  client: undefined,
  setClient: () => { },
  queryClient: null,
  setQueryClient: () => { },
  icaAddress: null,
  setICAAddress: () => { },
  icaControllerAddress: null,
  setICAControllerAddress: () => { },
});

export const GlobalContextProvider = ({ children }: Props) => {
  // the value that will be given to the context
  const [authenticated, setAuthenticated] = useState(false);
  const [userAddress, setUserAddress] = useState<string | null>(null);
  const [network, setNetwork] = useState<ChainConfig>(Networks["lst-network"]);
  const [client, setClient] = useState<SigningCosmWasmClient>();
  const [queryClient, setQueryClient] = useState();
  const [icaAddress, setICAAddress] = useState<string | null>(null);
  const [icaControllerAddress, setICAControllerAddress] = useState<string | null>(null);


  return (
    // the Provider gives access to the context to its children
    <GlobalContext.Provider
      value={{
        authenticated,
        setAuthenticated,

        userAddress,
        setUserAddress,

        network,
        setNetwork,

        client,
        setClient,

        queryClient,
        setQueryClient,

        icaAddress,
        setICAAddress,

        icaControllerAddress,
        setICAControllerAddress,

      }}
    >
      {children}
    </GlobalContext.Provider>
  );
};
