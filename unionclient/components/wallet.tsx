"use client";

import React, { useState, useEffect } from "react";
import { Dropdown, DropdownTrigger, DropdownMenu, DropdownItem, Button } from "@nextui-org/react";
import { LocalStorage } from "@/app/lib/localstorage";
import { useGlobalContext } from "@/app/core/context";
import {
  initializeKeplr,
  initializeLeap,
  initializeCosmos,
  setClientNomos,
} from "@/app/core/wallet";
import Networks from "@/config/networks.config";
import { CopyIcon, UserIcon } from "@/components/icons";
import { CosmWasmClient, SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";

export const truncateAddressSE = (address: any, start: number = 4, end: number = 4) => {
  try {
    if (!address) return "No Account";
    return `${address.slice(0, start)}…${address.slice(-end)}`;
  } catch (error) {
    return '';
  }
};

const copyToClipboard = async (text: string) => {
  try {
    await navigator.clipboard.writeText(text);
    alert('Address is copied to clipboard: ' + text);
  } catch (error) {
    alert('Error copying to clipboard:' + error);
  }
};

export const Wallet = () => {
  const {
    network,
    setClient,
    setUserAddress,
    setAuthenticated,
    authenticated,
    userAddress,
    setICAAddress,
    setQueryClient
  } = useGlobalContext();

  useEffect(() => {
    const initializeClient = async (
      setClient: (client: SigningCosmWasmClient) => void,
      setQueryClient: (client: CosmWasmClient) => void,
      setUserAddress: (userAddress: string | null) => void,
      setAuthenticated: (authenticated: boolean) => void
    ) => {
      if (LocalStorage.isAuthenticated() && LocalStorage.getWallet() != null) {
        const net = (Networks as any)[LocalStorage.getNetworkId() || "lst-network"];
        console.log({ net, key: LocalStorage.getNetworkId() || "lst-network" });
        await setClientNomos(
          (Networks as any)[LocalStorage.getNetworkId() || "lst-network"],
          LocalStorage.getWallet(),
          setClient,
          setQueryClient,
          setUserAddress,
          setAuthenticated
        );

      }
    };

    initializeClient(setClient, setQueryClient, setUserAddress, setAuthenticated);
  }, [setClient, setUserAddress, setAuthenticated]);

  async function disconnect() {
    (window as any).wallet = null;
    setAuthenticated(false);
    LocalStorage.setAuthenticated(false);

    setUserAddress(null);
    LocalStorage.setUserAddress(null);
  }

  interface WalletMenuProps {
    address: string | null | undefined;
  }

  const WalletMenuProfile = ({ address }: WalletMenuProps) => {
    return (
      <Dropdown>
        <DropdownTrigger>
          <Button variant="bordered" endContent={<UserIcon />}>
            {truncateAddressSE(address)}
          </Button>
        </DropdownTrigger>
        <DropdownMenu variant="faded" aria-label="Dropdown menu with icons" >
          <DropdownItem key="1" endContent={<CopyIcon />} onPress={() => { copyToClipboard(address ? address : "") }}>Copy address</DropdownItem>
          <DropdownItem key="2" onPress={disconnect}>Disconnect</DropdownItem>
        </DropdownMenu>
      </Dropdown>
    );
  };

  const WalletMenuConnect = () => {
    const [walletLoading, setWalletLoading] = useState(false);

    return (
      <Dropdown>
        <DropdownTrigger>
          <Button variant="bordered">{walletLoading ? "Loading ..." : "Connect wallet"}</Button>
        </DropdownTrigger>
        <DropdownMenu
          variant="faded"
          aria-label="Dropdown menu with icons"
          onAction={async (wallet: any) => {
            switch (wallet) {
              case "keplr":
                setWalletLoading(true);
                await initializeKeplr(network, setClient, setQueryClient, setUserAddress, setAuthenticated);
                setWalletLoading(false);
                break;
              case "leap":
                setWalletLoading(true);
                await initializeLeap(network, setClient, setQueryClient, setUserAddress, setAuthenticated);
                setWalletLoading(false);
                break;
              case "cosmostation":
                setWalletLoading(true);
                await initializeCosmos(network, setClient, setQueryClient, setUserAddress, setAuthenticated);
                setWalletLoading(false);
                break;
            }
          }}
        >
          <DropdownItem key="keplr">Keplr</DropdownItem>
          <DropdownItem key="leap">Leap</DropdownItem>
          <DropdownItem key="cosmostation">Cosmostation</DropdownItem>
        </DropdownMenu>
      </Dropdown>
    );
  };

  if (authenticated) {
    return <WalletMenuProfile address={userAddress} />;
  } else {
    return <WalletMenuConnect />;
  }
};

export default Wallet;
