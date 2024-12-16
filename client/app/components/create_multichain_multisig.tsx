"use client";

import {
  Card,
  CardBody,
  Button,
  Input,
  CardHeader,
  Spinner,
} from "@nextui-org/react";
import { useState } from "react";
import { useGlobalContext } from "@/app/core/context";
import React from "react";
import Networks, { SupportedICANetworks } from "@/config/networks.config";
import { CosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { LocalStorage } from "../lib/localstorage";

export default function CreateInterchainMultisig() {
  const {
    userAddress,
    client,
    network,
    interchainAccountID,
    multisigChain,
    icaAddress,
    queryClient,
    wallets,
    setWalletAddress,
    setWallets,
  } = useGlobalContext();
  const [totalMembers, setTotalMembers] = useState(1);
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: any) => {
    setLoading(true);
    e.preventDefault();
    const form = e.target;
    const formData = new FormData(form);
    const formEntries = Object.fromEntries(formData.entries());

    let members: any = [];
    let idx = 0;
    for (var key in formEntries) {
      console.log(formEntries[key]);
      if (key.includes("member-")) {
        members.push({
          addr: formEntries[key],
          weight: 1,
        });
      }
      if (key.includes("weight-")) {
        console.log(JSON.stringify(members));
        const weight = Number(formEntries[key]);
        members[idx]["weight"] = weight;
        idx += 1;
      }
    }
    const contractAddress = network?.contracts.factory.address;
    const targetContractAddress =
      Networks[multisigChain as SupportedICANetworks].contracts.factory.address;

    const msg = {
      create_interchain_multisig: {
        contract_addr: targetContractAddress,
        interchain_account_id: interchainAccountID,
        members,
        threshold: {
          absolute_count: {
            weight: Number(formEntries.threshold),
          },
        },
        max_voting_period: {
          time: Number(formEntries.duration),
        },
      },
    };

    try {
      const instantiateResult = await client.execute(
        userAddress,
        contractAddress,
        msg,
        "auto"
      );

      console.log(JSON.stringify(instantiateResult));
      await new Promise(resolve => setTimeout(resolve, 6000));
      await updateMultisigs();
      setWalletAddress(wallets[wallets.length - 1]);
    } catch (err) {
      console.log(err);
    }

    alert("Multisig is created successfully");
    setLoading(false);
  };

  const updateMultisigs = async () => {
    console.log("updateMultisigs");
    const icAddress: string = icaAddress ? icaAddress.toString() : "";
    const msg = { query_user_wallets: icAddress };
    const net = Networks[multisigChain as SupportedICANetworks];

    const res = await queryClient.queryContractSmart(
      net.contracts.factory.address,
      msg
    );

    LocalStorage.updateLocalMultisigs({
      networkId: multisigChain as string,
      userAddress: icAddress as string,
      wallets: res.wallets,
    });

    console.log("after create: ", res.wallets);
    setWalletAddress(res.wallets[-1]);
    setWallets(res.wallets);
  };

  const addMember = async (e: any) => {
    e.preventDefault();
    setTotalMembers(totalMembers + 1);
  };

  const MemberInput = ({ idx }: { idx: Number }) => {
    return (
      <div className="w-full  height-auto text-foreground box-border outline-none data-[focus-visible=true]:z-10 data-[focus-visible=true]:outline-2 data-[focus-visible=true]:outline-focus data-[focus-visible=true]:outline-offset-2 shadow-medium rounded-large transition-transform-background motion-reduce:transition-none border-transparent bg-white/5 dark:bg-default-400/10 backdrop-blur-lg backdrop-saturate-[1.8]">
        <div className="flex p-2 z-10 w-full justify-start items-center shrink-0 overflow-inherit color-inherit subpixel-antialiased rounded-t-large gap-1 pb-0">
          <p className="text-base font-semibold">Member {Number(idx) + 1}</p>
        </div>
        <div className="relative flex w-full p-2 flex-auto flex-col place-content-inherit align-items-inherit h-auto break-words text-left overflow-y-auto subpixel-antialiased gap-2">
          <Input
            key={Number(idx) + 1}
            isRequired
            name={`member-${idx}`}
            label="Address"
          />
          <Input
            key={Number(idx) + 2}
            isRequired
            name={`weight-${idx}`}
            label="Weight"
            className="hidden"
            defaultValue="1"
          />
        </div>
      </div>
    );
  };

  return (
    <div className="w-full gap-4">
      <Card>
        <CardHeader>Create Multichain Multisig</CardHeader>
        <CardBody className="gap-1">
          <form onSubmit={handleSubmit}>
            <div className="flex flex-row gap-2">
              <Button onClick={addMember}>Add Member (+)</Button>
              <Button onClick={() => setTotalMembers(1)}>Reset</Button>
            </div>
            <div className="w-full py-2">
              <Input
                isRequired
                name="interchain-account-id"
                label="Interchain Account ID"
                className="hidden"
                defaultValue={interchainAccountID as string}
              />
            </div>
            <div className="w-full flex flex-col gap-1 py-1">
              {Array.from(Array(totalMembers), (e, i) => {
                return <MemberInput key={i} idx={i} />;
              })}
            </div>
            <div className="w-full flex flex-col gap-1 py-1">
              <div className="flex p-2 z-10 w-full justify-start items-center shrink-0 overflow-inherit color-inherit subpixel-antialiased rounded-t-large gap-1 pb-0">
                <p className="text-base font-semibold">Threshold</p>
              </div>
              <Input
                isRequired
                name="threshold"
                label="Threshold"
                className="max-w-xs"
              />
              <Input
                isRequired
                name="duration"
                label="Duration (secs)"
                className="max-w-xs hidden"
                defaultValue="31536000"
              />
            </div>

            {!loading && <Button type="submit">Submit</Button>}

            {loading && (
              <div className="flex flex-row gap-2">
                <Button type="submit" isDisabled>
                  Submit
                </Button>
                <Spinner />
              </div>
            )}
          </form>
        </CardBody>
      </Card>
    </div>
  );
}
