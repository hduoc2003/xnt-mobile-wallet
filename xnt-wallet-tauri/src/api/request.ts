import axios, { type AxiosInstance, type AxiosResponse, type AxiosError } from 'axios'
import { RPC_URL, STATUS_CODE_SUCCESS } from '@/utils'

export const API_URL = RPC_URL

// Create axios instance
const instance: AxiosInstance = axios.create({
  baseURL: API_URL,
  timeout: 30000,
  headers: {
    'Content-Type': 'application/json',
  },
  withCredentials: false,
})

// Response interceptor
instance.interceptors.response.use(
  (response: AxiosResponse) => {
    if (response?.status !== STATUS_CODE_SUCCESS) {
      return Promise.reject(response?.data)
    }
    return response.data
  },
  (error: AxiosError) => {
    if (error?.response?.data) {
      return Promise.reject(error?.response?.data)
    }
    return Promise.reject(error)
  },
)

export default instance
