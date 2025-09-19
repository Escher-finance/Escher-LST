import { toHex } from "viem";

export function getSalt() {
    const rawSalt = new Uint8Array(32)
    crypto.getRandomValues(rawSalt)
    const salt = toHex(rawSalt);
    return salt
}

export function getTimeoutInNanoseconds7DaysFromNow(): bigint {
    const millisecondsNow = Date.now() // current time in ms
    const millisecondsIn7Days = 7 * 24 * 60 * 60 * 1000 * 3 // 24 hours in ms * 3
    const totalMilliseconds = millisecondsNow + millisecondsIn7Days;
    return BigInt(totalMilliseconds) * BigInt(1_000_000) // convert ms to ns
}


export function getTimestamp() {
    const current = Date.now();
    let timestamp = current + 900000;
    timestamp = timestamp * 10000000;
    return timestamp
}