import { err, ok, ResultAsync, type Result } from "neverthrow"
import { fromHex, Hex, isHex } from "viem";
import { request } from "graphql-request";
import { graphql } from "gql.tada";
import { CosmWasmClient } from "@cosmjs/cosmwasm-stargate";

const GRAQPHQL_URL = "https://staging.graphql.union.build/v1/graphql";


export function getTimestamp() {
    const current = Date.now();
    let timestamp = current + 900000;
    timestamp = timestamp * 10000000;
    return timestamp
}

const cosmosChainId: string[] = [
    "elgafar-1",
    "osmo-test-5",
    "union-testnet-9",
    "stride-internal-1",
    "bbn-test-5",
    "union-testnet-8"
];


export type CosmosChainId = `${(typeof cosmosChainId)[number]}`

export const cosmosRpcs: Record<CosmosChainId, string> = {
    "elgafar-1": "https://rpc.elgafar-1.stargaze.chain.kitchen",
    "osmo-test-5": "https://rpc.osmo-test-5.osmosis.chain.kitchen",
    "union-testnet-9": "https://rpc.union-testnet-9.union.chain.kitchen",
    "union-testnet-8": "https://rpc.union-testnet-8.union.chain.kitchen",
    "stride-internal-1": "https://rpc.stride-internal-1.stride.chain.kitchen",
    "bbn-test-5": "https://rpc.bbn-test-5.babylon.chain.kitchen"
}

export const getRecommendedChannels = async () => {
    const query = `query MyQuery {
                           v1_ibc_union_channel_recommendations(
                                where: 
                                {
                                source_chain_id: {_eq: "union-testnet-9"}
                                version: {_eq: "ucs03-zkgm-0"}
                                }
                            ) {
                                destination_chain_id
                                destination_channel_id
                                destination_port_id
                                destination_connection_id
                                source_chain_id
                                source_channel_id
                                source_connection_id
                                source_port_id
                            }
                        }
                `;


    const response = await fetch(GRAQPHQL_URL, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Accept': 'application/json',
        },
        body: JSON.stringify({
            query,
        })
    });

    if (!response.ok) {
        throw new Error(`Response status: ${response.status}`);
    }
    const res = await response.json();
    console.log(JSON.stringify(res));

    return res.data.v1_ibc_union_channel_recommendations;
}

export type Channel = {
    source_chain_id: string
    source_port_id: string
    source_channel_id: number
    source_connection_id: number
    destination_chain_id: string
    destination_port_id: string
    destination_channel_id: number
    destination_connection_id: number
}

export const getChannelInfo = (
    source_chain_id: string,
    destination_chain_id: string,
    channels: Awaited<ReturnType<typeof getRecommendedChannels>>
): Channel | null => {
    let rawChannel = channels.find(
        (chan: any) =>
            chan.source_chain_id === source_chain_id && chan.destination_chain_id === destination_chain_id
    )
    if (
        // Validate that all required fields are included by the garphql api.
        !rawChannel ||
        rawChannel.source_connection_id === null ||
        rawChannel.source_channel_id === null ||
        !rawChannel.source_port_id ||
        rawChannel.destination_connection_id === null ||
        rawChannel.destination_channel_id === null ||
        !rawChannel.destination_port_id
    ) {
        return null
    }

    let source_port_id = String(rawChannel.source_port_id)
    if (source_port_id.length < 4) return null
    source_port_id = source_port_id.slice(2)

    let destination_port_id = String(rawChannel.destination_port_id)
    if (destination_port_id.length < 4) return null
    destination_port_id = destination_port_id.slice(2)

    return {
        source_chain_id,
        source_connection_id: rawChannel.source_connection_id,
        source_channel_id: rawChannel.source_channel_id,
        source_port_id,
        destination_chain_id,
        destination_connection_id: rawChannel.destination_connection_id,
        destination_channel_id: rawChannel.destination_channel_id,
        destination_port_id
    }
}

const tokenWrappingQuery = graphql(/*  GraphQL */ `
    query QueryTokenWrapping(
      $source_chain_id: String!
      $base_token: String!
      $destination_channel_id: Int!
    ) {
      v1_ibc_union_tokens(where: {_and: {chain: {chain_id: {_eq: $source_chain_id}}, denom: {_eq: $base_token}, wrapping: {_and: {index: {_eq: 0}, destination_channel_id: {_eq: $destination_channel_id}}}}}) {
        denom
        wrapping {
          destination_channel_id
          unwrapped_chain {
            chain_id
          }
          unwrapped_address_hex
        }
      }
    }
    
    `)



export const getQuoteToken = async (
    source_chain_id: string,
    base_token: Hex,
    channel: Channel
): Promise<
    Result<
        { quote_token: string; type: "UNWRAPPED" | "NEW_WRAPPED" } | { type: "NO_QUOTE_AVAILABLE" },
        Error
    >
> => {
    // check if the denom is wrapped
    let wrapping: any = await ResultAsync.fromPromise(
        request(GRAQPHQL_URL, tokenWrappingQuery, {
            base_token,
            destination_channel_id: channel.source_channel_id, // not a mistake! because we're unwrapping
            source_chain_id
        }),
        error => {
            console.error("@unionlabs/client-[getQuoteToken]", error)
            return new Error("failed to get quote token from graphql", { cause: error })
        }
    )

    if (wrapping.isErr()) {
        return err(wrapping.error)
    }

    let wrappingTokens = wrapping.value.v1_ibc_union_tokens

    // if it is, quote token is the unwrapped verison of the wrapped token.
    if (wrappingTokens.length > 0) {
        let quote_token = wrappingTokens.at(0)?.wrapping.at(0)?.unwrapped_address_hex
        if (quote_token) {
            return ok({ type: "UNWRAPPED", quote_token })
        }
    }

    // if it is unknown, calculate the quotetoken
    // cosmos quote token prediction
    if (cosmosChainId.includes(channel.destination_chain_id)) {
        let rpc = cosmosRpcs[channel.destination_chain_id as CosmosChainId] // as is valid bc of the check in the if statement.

        let publicClient = await ResultAsync.fromPromise(CosmWasmClient.connect(rpc), error => {
            return new Error(`failed to create public cosmwasm client with rpc ${rpc}`, { cause: error })
        })

        if (publicClient.isErr()) {
            return err(publicClient.error)
        }

        let client = publicClient.value

        let predictedQuoteToken = await ResultAsync.fromPromise(
            client.queryContractSmart(fromHex(`0x${channel.destination_port_id}`, "string"), {
                predict_wrapped_token: {
                    path: "0",
                    channel: channel.destination_channel_id,
                    token: base_token
                }
            }),
            error => {
                return new Error("failed to query predict wrapped denom", { cause: error })
            }
        ).andThen(res =>
            res?.wrapped_token && isHex(res?.wrapped_token) && res?.wrapped_token.length > 2
                ? ok(res.wrapped_token)
                : err(new Error(`no wrapped token in response: ${JSON.stringify(res)}`))
        )

        if (predictedQuoteToken.isErr()) {
            return err(predictedQuoteToken.error)
        }

        return ok({ type: "NEW_WRAPPED", quote_token: predictedQuoteToken.value })
    }

    return err(new Error("unknown chain in token prediction"))
}