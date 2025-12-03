import request from './request'
import { invoke } from '@tauri-apps/api/core'

const DEFAULT_MIN_BLOCK_HEIGHT = 0

const isTauriEnv =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in (window as any)

export const callJsonRpc = async (method: string, params: any = []) => {
  const requestData = {
    jsonrpc: '2.0',
    method,
    params,
    id: 1,
  }

  if (isTauriEnv) {
    const result = await invoke<string>('proxy_json_rpc', {
      method,
      params: requestData.params,
    })
    return JSON.parse(result)
  }

  return request({
    method: 'POST',
    url: '/', // explicit root path on JSON-RPC server
    data: requestData,
  })
}

/**
 * Get UTXOs from view key
 */
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

/**
 * Get balance from view key
 */
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

/**
 * Get current block height
 */
export const getBlockHeight = async (): Promise<any> => {
  return await callJsonRpc('chain_height')
}

/**
 * Get network info
 */
export const getNetworkInfo = async (): Promise<any> => {
  return await callJsonRpc('node_network')
}

/**
 * Broadcast signed transaction
 */
export const broadcastSignedTransaction = async (transactionHex: string): Promise<any> => {
  const params = {
    transactionHex,
  }
  return await callJsonRpc('mempool_submitTransaction', params)
}

