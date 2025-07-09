"use client";

import Bond from "./bond";
import Unbond from "./unbond";
import Assets from "./assets";
import { Tabs, Tab, Chip, Link } from "@nextui-org/react";
import UnbondingRecords from "./unbonding_records";
import RewardAsset from "./reward_asset";
import RevenueAssets from "./revenue_assets";
import { useState } from "react";
import Liquidity from "./liquidity";
import TransactionHistory from "./transaction_history";
import { useGlobalContext } from "@/app/core/context";
import IbcBond from "./ibc_bond";
import ZkgmUnbond from "./zkgm_unbond";

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
          <Chip><Link color="warning" onPress={refresh}>Refresh</Link></Chip>
        </div>
        {network?.chainId.indexOf("osmo") == -1 &&
          <div className="flex flex-col">

            <div className="grid grid-cols-2 gap-4">
              <div className="flex flex-col gap-2">
                <Assets stateKey={stateKey} />
                <RevenueAssets stateKey={stateKey} />
                <RewardAsset stateKey={stateKey} />
              </div>
              <Liquidity stateKey={stateKey} />
            </div>

            <Tabs aria-label="Tabs sizes" className="mt-10 p-3">
              <Tab key="bond" title="Bond">
                <Bond stateKey={stateKey} setStateKey={setStateKey} />
              </Tab>


              <Tab key="unbond" title="Unbond" >
                <Unbond stateKey={stateKey} setStateKey={setStateKey} />
              </Tab>
              <Tab key="records" title="Unbonding Process">
                <UnbondingRecords />
              </Tab>
              <Tab key="history" title="Transactions History">
                <TransactionHistory />
              </Tab>
            </Tabs>

          </div>
        }

        {network?.chainId.indexOf("osmo") != -1 &&
          <>
            <IbcBond stateKey={stateKey} setStateKey={setStateKey} />
            <br />
            <ZkgmUnbond stateKey={stateKey} setStateKey={setStateKey} />
          </>
        }

      </div>
    </div >
  );
}
