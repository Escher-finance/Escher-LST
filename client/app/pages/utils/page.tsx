"use client";

import InstantiateCW20 from "./instantiate_cw20";
import InstantiateICA from "./instantiate_ica";
import ContractUpload from "./upload";
import InstantiateLiquidStaking from "./instantiate_lst";
import ExecuteBond from "./execute_bond";
import { Accordion, AccordionItem } from "@nextui-org/react";
import QueryTotalBond from "./query_total_bond";
import QueryConfig from "./query_config";
import SetAdmin from "./set_admin";
import Mint from "./mint";
import TransferEVM from "./transfer_evm";
import Migrate from "./migrate";
import Unbond from "./unbond";
import BondRewards from "./bond_rewards";
import DecodePacket from "./decode";
import InstantiateAuthz from "./instantiate_authz";
import SetParams from "./set_params";
import SetMinter from "./set_minter";
import InstantiateReward from "./instantiate_reward";
import SetReward from "./set_reward";
import MoveToReward from "./move_to_reward";

export default function Utils() {
  return (
    <div className="w-full flex flex-col gap-4">
      <div className="w-full">
        <h1>Utilities</h1>
      </div>
      <div className="w-full flex flex-row gap-4">
        <div className="w-full flex flex-col">
          <Accordion variant="splitted">
            <AccordionItem
              key="1"
              aria-label="Upload Contract"
              title="Upload Contract"
            >
              <ContractUpload />
            </AccordionItem>
            <AccordionItem
              key="2"
              aria-label="InstantiateICA"
              title="InstantiateICA"
            >
              <InstantiateICA />
            </AccordionItem>
            <AccordionItem
              key="3"
              aria-label="InstantiateCW20 Token"
              title="Instantiate CW20"
            >
              <InstantiateCW20 />
            </AccordionItem>
            <AccordionItem
              key="4"
              aria-label="Instantiate LiquidStaking"
              title="Instantiate LiquidStaking"
            >
              <InstantiateLiquidStaking />
            </AccordionItem>
            <AccordionItem
              key="5"
              aria-label="Execute Bond"
              title="Execute Bond"
            >
              <ExecuteBond />
            </AccordionItem>
            <AccordionItem
              key="6"
              aria-label="Total Bond"
              title="Total Bond"
            >
              <QueryTotalBond />
            </AccordionItem>
            <AccordionItem
              key="7"
              aria-label="Config"
              title="Config"
            >
              <QueryConfig />
            </AccordionItem>
            <AccordionItem
              key="8"
              aria-label="Set Token Admin"
              title="Set Token Admin"
            >
              <SetAdmin />
            </AccordionItem>
            <AccordionItem
              key="9"
              aria-label="Mint"
              title="Mint"
            >
              <Mint />
            </AccordionItem>
            <AccordionItem
              key="10"
              aria-label="Transfer to EVM"
              title="Transfer to EVM"
            >
              <TransferEVM />
            </AccordionItem>
            <AccordionItem
              key="11"
              aria-label="Unbond"
              title="Unbond"
            >
              <Unbond />
            </AccordionItem>
            <AccordionItem
              key="12"
              aria-label="Migrate"
              title="Migrate"
            >
              <Migrate />
            </AccordionItem>
            <AccordionItem
              key="13"
              aria-label="Bond Rewards"
              title="Bond Rewards"
            >
              <BondRewards />
            </AccordionItem>
            <AccordionItem
              key="14"
              aria-label="Decode"
              title="Decode"
            >
              <DecodePacket />
            </AccordionItem>
            <AccordionItem
              key="15"
              aria-label="Instantiate Authz"
              title="Instantiate Authz"
            >
              <InstantiateAuthz />
            </AccordionItem>

            <AccordionItem
              key="16"
              aria-label="Set Parameters"
              title="Set Parameters"
            >
              <SetParams />
            </AccordionItem>
            <AccordionItem
              key="17"
              aria-label="Set Minter"
              title="Set Minter"
            >
              <SetMinter />
            </AccordionItem>
            <AccordionItem
              key="18"
              aria-label="Instantiate Reward"
              title="Instantiate Reward"
            >
              <InstantiateReward />
            </AccordionItem>
            <AccordionItem
              key="19"
              aria-label="Set Reward"
              title="Set Reward"
            >
              <SetReward />
            </AccordionItem>
            <AccordionItem
              key="20"
              aria-label="Move To Reward"
              title="Move To Reward"
            >
              <MoveToReward />
            </AccordionItem>
          </Accordion>
        </div>
      </div>
    </div>
  );
}
