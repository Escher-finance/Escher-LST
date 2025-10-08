
import { bech32, hex, bytes } from "@scure/base"

export type Hex = `0x${string}`


/**
 * raise a runtime error
 * @example
 * ```ts
 * raise("something went wrong")
 * raise(new Error("something went wrong"))
 * ```
 */
export function raise(error: unknown): never {
    throw typeof error === "string" ? new Error(error) : error
}


/** Hex address of the form `0x${string}`. Used for EVM addresses. */
export type HexAddress = `0x${string}`

/** Bech32 address of the form `${string}1${string}`. Used for Cosmos addresses. */
export type Bech32Address<T extends string = string> = `${T}1${string}`

export function hexToBytes(hexString: string): Uint8Array {
    return bytes("hex", hexString.indexOf("0x") === 0 ? hexString.slice(2) : hexString)
}

export function hexAddressToBech32({
    address,
    bech32Prefix
}: { address: HexAddress; bech32Prefix: string }): Bech32Address {
    const words = bech32.toWords(hexToBytes(address))
    return bech32.encode(bech32Prefix, words, false)
}

export function bech32AddressToHex({ address }: { address: string }): HexAddress {
    if (!isValidBech32ContractAddress(address)) raise(`Invalid bech32 address: ${address}`)
    const { bytes } = bech32.decodeToBytes(address)
    return `0x${bytesToHex(bytes)}`
}


/**
 * check if a string is a valid bech32 contract address
 * @example
 * ```ts
 * isValidBech32ContractAddress("union14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sgf2v9u")
 * ```
 */
export function isValidBech32ContractAddress(address: unknown): address is Bech32Address {
    if (typeof address !== "string") return false

    try {
        const { prefix: _, words } = bech32.decode(address as Bech32Address)
        // Contract addresses can have variable lengths, we just verify it's a valid bech32 encoding
        return true
    } catch {
        return false
    }
}

/**
 * convert a byte array to a hex string
 * @example
 * ```ts
 * bytesToHex(new Uint8Array([1, 2, 3]))
 * ```
 */
export function bytesToHex(bytes: Uint8Array): string {
    return hex.encode(bytes)
}

