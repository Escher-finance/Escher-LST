import {
    MsgExecuteContract,
  } from 'cosmjs-types/cosmwasm/wasm/v1/tx';
  import {toUtf8} from '@cosmjs/encoding';

export const getExecuteContractMessage = (sender: string, contract_addr: string, msg: any, funds: any[]) => {
    return {
        typeUrl: '/cosmwasm.wasm.v1.MsgExecuteContract',
        value: MsgExecuteContract.fromPartial({
        sender,
        contract: contract_addr,
        msg: toUtf8(JSON.stringify(msg)),
        funds,
        })  
    }
};
