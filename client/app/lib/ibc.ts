


export const createSendIBCMsg = ({
    sender,
    denom,
    amount,
    sourceChannel,
    receiver,
    timeoutTimestamp,
    memo
}: {
    sender: string;
    denom: string;
    amount: string;
    sourceChannel: string;
    receiver: string;
    timeoutTimestamp: BigInt;
    memo: string;
}) => {
    const typeUrl = "/ibc.applications.transfer.v1.MsgTransfer";
    return {
        typeUrl,
        value: {
            sourcePort: "transfer",
            sourceChannel,
            token: {
                amount,
                denom,
            },
            sender,
            receiver,
            timeoutTimestamp,
            memo,
        },
    };
};

export function getTimeoutInNanoseconds24HoursFromNow(): bigint {
    const millisecondsNow = Date.now() // current time in ms
    const millisecondsIn24Hours = 24 * 60 * 60 * 1000 * 3 // 24 hours in ms * 3
    const totalMilliseconds = millisecondsNow + millisecondsIn24Hours
    return BigInt(totalMilliseconds) * BigInt(1_000_000) // convert ms to ns
}