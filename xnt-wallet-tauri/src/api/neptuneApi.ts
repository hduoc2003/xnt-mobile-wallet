import request from './request'
import { invoke } from '@tauri-apps/api/core'

const isTauriEnv = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in (window as any)

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
    url: '/',
    data: requestData,
  })
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
export const broadcastSignedTransaction = async (transaction: string): Promise<any> => {
  const params = {
    transaction,
  }
  return await callJsonRpc('mempool_submitTransaction', params)
}

export const getXntPrice = async (url: string): Promise<any> => {
  return await JSON.parse(await invoke<string>('proxy_http_get', { url }))
}
