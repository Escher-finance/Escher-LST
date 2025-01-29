const hexes = /*#__PURE__*/ Array.from({ length: 256 }, (_v, i) =>
    i.toString(16).padStart(2, '0'),
)

export function toHex(value: Uint8Array): string {
    let string = ''
    for (let i = 0; i < value.length; i++) {
        string += hexes[value[i]]
    }
    const hex = `0x${string}`;
    return hex;
}


export function getSalt() {
    const rawSalt = new Uint8Array(32)
    crypto.getRandomValues(rawSalt)
    const salt = toHex(rawSalt);
    return salt
}