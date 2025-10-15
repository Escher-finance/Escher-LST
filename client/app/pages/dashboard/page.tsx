"use client";

import Bond from "./bond";
import Unbond from "./unbond";
import Assets from "./assets";
import { Tabs, Tab, Chip, Link } from "@heroui/react";
import RewardAsset from "./reward_asset";
import RevenueAssets from "./revenue_assets";
import { useState } from "react";
import Liquidity from "./liquidity";
import { useGlobalContext } from "@/app/core/context";
import IbcBond from "./ibc_bond";
import TransferU from "./transfer_u";
import TransfereU from "./transfer_eu";
import UnionBond from "./union_bond";
import UnionInfo from "./union_info";
import TransferFromBabylon from "./transfer_from_babylon";
import UnionUnbond from "./union_unbond";
import TransferBaby from "./transfer_baby";
//import ZkgmUnbond from "./zkgm_unbond";

export default function Dashboard() {
  const { network } = useGlobalContext();

  const [stateKey, setStateKey] = useState(1);

  const refresh = async () => {
    let newKey = stateKey + 1;
    setStateKey(newKey);
  }

  return (
    <div className="w-full flex flex-col gap-2">
      <div className="w-full mt-2 ">
        <div className="flex flex-row items-center pb-2">
          <h1 className="p-3 text-2xl">Escher Liquid Staking</h1>
          <Chip><Link onPress={refresh}>Refresh</Link></Chip>
        </div>
        {network?.chainId.indexOf("bbn") != -1 &&
          <div className="flex flex-col">

            <div className="grid grid-cols-2 gap-4">
              <div className="flex flex-col gap-2">
                <Assets stateKey={stateKey} />
                <RevenueAssets stateKey={stateKey} />
                <RewardAsset stateKey={stateKey} />
              </div>
              <Liquidity stateKey={stateKey} />
            </div>

            <Tabs className="mt-10 p-3">
              <Tab key="bond" title="Bond">
                <Bond stateKey={stateKey} setStateKey={setStateKey} />
              </Tab>


              <Tab key="unbond" title="Unbond" >
                <Unbond stateKey={stateKey} setStateKey={setStateKey} />
              </Tab>

              <Tab key="transfer" title="Transfer Baby or eBaby" >
                <TransferFromBabylon stateKey={stateKey} setStateKey={setStateKey} />
              </Tab>

              <Tab key={"transfer_baby_via_contract"} title="Transfer Baby via Contract" >
                <TransferBaby stateKey={stateKey} setStateKey={setStateKey} />
              </Tab>
            </Tabs>

          </div>
        }

        {network?.chainId.indexOf("osmo") != -1 &&
          <Tabs className="mt-10 p-3">
            <Tab key="ibc_bond" title="IbcBond" >
              <IbcBond stateKey={stateKey} setStateKey={setStateKey} />
            </Tab>
            {/* <Tab key="zkgm_unbond" title="ZKGM Unbond" >
              <ZkgmUnbond stateKey={stateKey} setStateKey={setStateKey} />
            </Tab> */}

          </Tabs>
        }

        {network?.chainId.indexOf("union") != -1 &&

          <div>
            <div>
              <UnionInfo stateKey={stateKey} />
            </div>
            <Tabs className="mt-10 p-1">
              <Tab key="bond" title="Bond">
                <UnionBond stateKey={stateKey} setStateKey={setStateKey} />
              </Tab>
              <Tab key="unbond" title="Unbond">
                <UnionUnbond stateKey={stateKey} setStateKey={setStateKey} />
              </Tab>
              <Tab key="transfer_u" title="Transfer U" >
                <TransferU stateKey={stateKey} setStateKey={setStateKey} />
              </Tab>
              <Tab key="transfer_eu" title="Transfer eU" >
                <TransfereU stateKey={stateKey} setStateKey={setStateKey} />
              </Tab>

            </Tabs>

          </div>
        }

      </div >
    </div>
  );
}
