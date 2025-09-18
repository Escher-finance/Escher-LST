"use client";

import React, { useEffect, useState } from "react";
import {
  Dropdown,
  DropdownTrigger,
  DropdownMenu,
  DropdownItem,
  Button,
} from "@heroui/react";
import { BaseNetworks, ChainConfig } from "@/config/networks.config";
import { useGlobalContext } from "@/app/core/context";
import { LocalStorage } from "@/app/lib/localstorage";
export const NetworkSwitch = () => {
  const { network, setNetwork, setAuthenticated, setUserAddress,  } =
    useGlobalContext();

  const [networkId, setNetworkId] = useState("");

  const setNetworkChain = (chainId: any) => {
    let chain: ChainConfig = (BaseNetworks as any)[chainId];
    setNetwork(chain);
    console.log(chain.chainName);
    (window as any).wallet = null;
    setAuthenticated(false);
    LocalStorage.setAuthenticated(false);

    setUserAddress(null);
    LocalStorage.setUserAddress(null);
    LocalStorage.setNetworkId(chainId);
  };

  useEffect(() => {
    const networkID = LocalStorage.getNetworkId();

    if (networkID && networkID in BaseNetworks) {
      const netConfig = BaseNetworks[networkID as keyof typeof BaseNetworks];
      setNetwork(netConfig);
      
    }
  });

  return (
    <Dropdown>
      <DropdownTrigger>
        <Button variant="bordered">{network?.chainName}</Button>
      </DropdownTrigger>
      <DropdownMenu
        variant="faded"
        aria-label="Dropdown menu with icons"
        onAction={setNetworkChain}
      >
        {Object.keys(BaseNetworks).map((chainId) => (
          <DropdownItem key={chainId}>
            {BaseNetworks[chainId as keyof typeof BaseNetworks].chainName}
          </DropdownItem>
        ))}
      </DropdownMenu>
    </Dropdown>
  );
};

export default NetworkSwitch;
