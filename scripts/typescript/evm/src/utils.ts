import { toHex } from "viem";

export function getSalt() {
    const rawSalt = new Uint8Array(32)
    crypto.getRandomValues(rawSalt)
    const salt = toHex(rawSalt);
    return salt
}

export function getTimeoutInNanoseconds7DaysFromNow(): bigint {
    const millisecondsNow = Date.now() // current time in ms
    const millisecondsIn24Hours = 24 * 60 * 60 * 1000 * 7 // 24 hours in ms * 7
    const totalMilliseconds = millisecondsNow + millisecondsIn24Hours
    return BigInt(totalMilliseconds) * BigInt(1_000_000) // convert ms to ns
}

