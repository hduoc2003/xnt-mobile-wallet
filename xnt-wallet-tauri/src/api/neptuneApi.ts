import { callJsonRpc } from '@/api/request'
import { DEFAULT_MIN_BLOCK_HEIGHT } from '@/utils/constants/constants'

export const getUtxosFromViewKey = async (
    viewKey: string,
    startBlock: number = DEFAULT_MIN_BLOCK_HEIGHT,
    endBlock: number | null = null,
    maxSearchDepth: number = 1000
): Promise<any> => {
    const params = {
        viewKey,
        startBlock,
        endBlock,
        maxSearchDepth,
    }
    return await callJsonRpc('wallet_getUtxoInfo', params)
}

export const getBalance = async (
    viewKey: string,
    startBlock: number = DEFAULT_MIN_BLOCK_HEIGHT,
    endBlock: number | null = null,
    maxSearchDepth: number = 1000
): Promise<any> => {
    const params = {
        viewKey,
        startBlock,
        endBlock,
        maxSearchDepth,
    }
    return await callJsonRpc('wallet_getBalanceFromViewKey', params)
}

export const getBlockHeight = async (): Promise<any> => {
    return await callJsonRpc('chain_height')
}

export const getNetworkInfo = async (): Promise<any> => {
    return await callJsonRpc('node_network')
}

export const broadcastSignedTransaction = async (transactionHex: any): Promise<any> => {
    const params = {
        transactionHex,
    }
    return await callJsonRpc('mempool_submitTransaction', params)
}
