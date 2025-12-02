import request from '@/api/config'
import { API_URL } from '@/api/config'

export const callJsonRpc = (method: string, params: any = []) => {
    const requestData = {
        jsonrpc: '2.0',
        method,
        params,
        id: 1,
    }
    
    const fullUrl = `${API_URL}`
    const requestLog = {
        url: fullUrl,
        method: 'POST',
        jsonrpc: requestData.jsonrpc,
        method_name: method,
        params: params,
        id: requestData.id,
    }
    
    if ((window as any).logger?.debug) {
        ;(window as any).logger.debug(
            `[JSON-RPC] → Request: ${method}\n` +
            `Full URL: ${fullUrl}\n` +
            `Request: ${JSON.stringify(requestLog, null, 2)}`
        )
    }
    
    return request({
        method: 'POST',
        url: '/',
        data: requestData,
    }).then((response) => {
        const responseLog = {
            url: fullUrl,
            method: method,
            response: response,
        }
        
        if ((window as any).logger?.debug) {
            ;(window as any).logger.debug(
                `[JSON-RPC] ← Response: ${method}\n` +
                `Full URL: ${fullUrl}\n` +
                `Response: ${JSON.stringify(responseLog, null, 2)}`
            )
        }
        
        return response
    }).catch((error) => {
        const errorLog = {
            url: fullUrl,
            method: method,
            error_message: error?.message || String(error),
            error_code: error?.code,
            error_response: error?.response?.data,
            error_status: error?.response?.status,
            error_statusText: error?.response?.statusText,
            error_headers: error?.response?.headers,
            error_config: {
                url: error?.config?.url,
                baseURL: error?.config?.baseURL,
                method: error?.config?.method,
                data: error?.config?.data,
            },
        }
        
        if ((window as any).logger?.error) {
            ;(window as any).logger.error(
                `[JSON-RPC] ✖ Error: ${method}\n` +
                `Full URL: ${fullUrl}\n` +
                `Error Details: ${JSON.stringify(errorLog, null, 2)}`
            )
        }
        throw error
    })
}
