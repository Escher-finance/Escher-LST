"use client";

import ExecuteBond from "../utils/execute_bond";
import Unbond from "../utils/unbond";
import Assets from "./assets";
import { Tabs, Tab } from "@nextui-org/react";
import UnbondingRecords from "./unbonding_records";
// import Liquidity from "./liquidity";
// import ContractAssets from "./contract_assets";
import RewardAsset from "./reward_asset";
import RevenueAssets from "./revenue_assets";
import { useState } from "react";
import Liquidity from "./liquidity";

export default function Dashboard() {


  const [stateKey, setStateKey] = useState(1);

  return (
    <div className="w-full flex flex-col gap-2">
      <div className="w-full mt-2">
        <h1 className="p-3 text-2xl mb-5">Escher Liquid Staking</h1>
        <div className="flex flex-col">

          <div className="grid grid-cols-2 gap-4">
            <div className="flex flex-col gap-4">
              <div className="flex flex-row gap-4">
                <Assets stateKey={stateKey} />
                <RevenueAssets stateKey={stateKey} />
              </div>
              <div className="flex flex-row gap-4">
                <RewardAsset stateKey={stateKey} />
                {/*<ContractAssets stateKey={stateKey} /> */}
              </div>

            </div>

            <Liquidity stateKey={stateKey} />
          </div>

          <Tabs aria-label="Tabs sizes" className="mt-10 p-3">
            <Tab key="bond" title="Bond">
              <ExecuteBond stateKey={stateKey} setStateKey={setStateKey} />
            </Tab>
            <Tab key="unbond" title="Unbond" >
              <Unbond stateKey={stateKey} setStateKey={setStateKey} />
            </Tab>
          </Tabs>

        </div>

      </div>
    </div>
  );
}
