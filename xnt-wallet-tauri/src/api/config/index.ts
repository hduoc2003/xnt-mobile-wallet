import { STATUS_CODE_SUCCESS } from '@/utils'
import axios from 'axios'

axios.defaults.withCredentials = false
export const API_URL = import.meta.env.VITE_APP_API || 'http://zsmartex.com:9897'

const instance = axios.create({
    baseURL: API_URL,
    headers: {
        'Content-Type': 'application/json',
    },
})

instance.interceptors.request.use(
    function (config) {
        const fullUrl = `${config.baseURL || ''}${config.url || ''}`
        const requestDetails = {
            url: fullUrl,
            baseURL: config.baseURL,
            method: config.method,
            headers: config.headers,
            data: config.data,
            params: config.params,
        }
        
        if ((window as any).logger?.debug) {
            ;(window as any).logger.debug(
                `[HTTP] → Request Interceptor\n` +
                `Full URL: ${fullUrl}\n` +
                `Request Details: ${JSON.stringify(requestDetails, null, 2)}`
            )
        }
        
        return config
    },
    function (error) {
        if ((window as any).logger?.error) {
            ;(window as any).logger.error(
                `[HTTP] → Request Error: ${JSON.stringify(error, null, 2)}`
            )
        }
        return Promise.reject(error)
    }
)

instance.interceptors.response.use(
    function (response) {
        const fullUrl = `${response.config?.baseURL || ''}${response.config?.url || ''}`
        const responseDetails = {
            url: fullUrl,
            status: response.status,
            statusText: response.statusText,
            headers: response.headers,
            data: response.data,
            config: {
                method: response.config?.method,
                baseURL: response.config?.baseURL,
                url: response.config?.url,
            },
        }
        
        if ((window as any).logger?.debug) {
            ;(window as any).logger.debug(
                `[HTTP] ← Response Interceptor\n` +
                `Full URL: ${fullUrl}\n` +
                `Status: ${response.status} ${response.statusText}\n` +
                `Response Details: ${JSON.stringify(responseDetails, null, 2)}`
            )
        }
        
        if (response?.status !== STATUS_CODE_SUCCESS) return Promise.reject(response?.data)
        return response.data
    },
    function (error) {
        const fullUrl = error?.config ? `${error.config.baseURL || ''}${error.config.url || ''}` : 'unknown'
        const errorDetails = {
            url: fullUrl,
            message: error?.message,
            code: error?.code,
            response: {
                status: error?.response?.status,
                statusText: error?.response?.statusText,
                headers: error?.response?.headers,
                data: error?.response?.data,
            },
            config: {
                url: error?.config?.url,
                baseURL: error?.config?.baseURL,
                method: error?.config?.method,
                data: error?.config?.data,
            },
        }
        
        if ((window as any).logger?.error) {
            ;(window as any).logger.error(
                `[HTTP] ← Response Error\n` +
                `Full URL: ${fullUrl}\n` +
                `Error Details: ${JSON.stringify(errorDetails, null, 2)}`
            )
        }
        
        if (error?.response?.data) {
            return Promise.reject(error?.response?.data)
        }
        return Promise.reject(error)
    }
)

export const setLocaleApi = (locale: string) => {
    instance.defaults.headers.common['lang'] = locale
}

export default instance
