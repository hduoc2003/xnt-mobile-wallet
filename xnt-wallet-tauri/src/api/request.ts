import axios, { type AxiosInstance, type AxiosResponse, type AxiosError } from 'axios'
import { STATUS_CODE_SUCCESS } from '@/utils/constants'

export const API_URL = import.meta.env.VITE_APP_API || 'http://zsmartex.com:9897'

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
  }
)

export default instance
