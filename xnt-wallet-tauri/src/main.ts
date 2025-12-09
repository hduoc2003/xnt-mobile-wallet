import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import i18n from '@/i18n'
import App from './App.vue'
import router from './router'
import './style.css'

const safeInvoke = async <T = any>(cmd: string, args?: any): Promise<T> => {
  const maxWait = 5000
  const startTime = Date.now()

  // eslint-disable-next-line no-constant-condition
  while (true) {
    try {
      return await invoke(cmd, args)
    } catch (e: any) {
      const msg = String(e?.message || e || '')
      const looksLikeNotReady =
        msg.includes("Cannot read properties of undefined (reading 'invoke')") ||
        msg.includes('__TAURI_INTERNALS__') ||
        msg.includes('Tauri API not available') ||
        msg.includes('Tauri is not ready')
      const elapsed = Date.now() - startTime

      if (!looksLikeNotReady || elapsed >= maxWait) {
        throw new Error(msg || 'Tauri API not available')
      }

      await new Promise((resolve) => setTimeout(resolve, 50))
    }
  }
}

const sendLogToNative = (level: 'trace' | 'debug' | 'info' | 'warn' | 'error', message: string) => {
    try {
        safeInvoke('log_message', { level, message }).catch(() => {})
    } catch {
        // Silent fail
    }
}

if (!(window as any).logger) {
    (window as any).logger = {
        info: (...args: any[]) => {
            const message = args.map(a => typeof a === 'string' ? a : JSON.stringify(a)).join(' ')
            sendLogToNative('info', message)
        },
        warn: (...args: any[]) => {
            const message = args.map(a => typeof a === 'string' ? a : JSON.stringify(a)).join(' ')
            sendLogToNative('warn', message)
        },
        error: (...args: any[]) => {
            const message = args.map(a => typeof a === 'string' ? a : JSON.stringify(a)).join(' ')
            console.error('[ERROR]', ...args)
            sendLogToNative('error', message)
        },
        debug: (...args: any[]) => {
            const message = args.map(a => typeof a === 'string' ? a : JSON.stringify(a)).join(' ')
            sendLogToNative('debug', message)
        },
    }
}

if (typeof window !== 'undefined') {
    window.addEventListener('error', (event) => {
        const msg = `[JS error] ${event.message || ''} at ${event.filename}:${event.lineno}:${event.colno}`
        console.error(msg, event.error)
        sendLogToNative('error', msg)
    })

    window.addEventListener('unhandledrejection', (event: PromiseRejectionEvent) => {
        const reason = (event.reason && (event.reason.message || event.reason.toString())) || 'unknown'
        const msg = `[JS unhandledrejection] ${reason}`
        console.error(msg, event.reason)
        sendLogToNative('error', msg)
    })
}

if (!(window as any).walletApi) {
    ;(window as any).walletApi = {
        generateSeed: async () => {
            try {
                const result = await safeInvoke<string>('generate_seed')
                return JSON.parse(result)
            } catch (error: any) {
                throw new Error(error?.message || error || 'Failed to generate seed')
            }
        },
        
        generateKeysFromSeed: async (seedPhrase: string[], network?: string) => {
            try {
                let networkStr = network || 'main'
                if (networkStr === 'mainnet') {
                    networkStr = 'main'
                }
                const result = await safeInvoke<string>('generate_keys_from_seed', { 
                    seedPhrase,
                    network: networkStr
                })
                return JSON.parse(result)
            } catch (error: any) {
                console.error('[walletApi] Error generating keys:', error)
                throw new Error(error?.message || error || 'Failed to generate keys from seed')
            }
        },
        
        decryptKeystore: async (keystoreFileName: string, password: string) => {
            try {
                const result = await safeInvoke<string>('decrypt_keystore', { 
                    fileName: keystoreFileName, 
                    password 
                })
                return JSON.parse(result)
            } catch (error: any) {
                return { success: false, error: error || 'Failed to decrypt keystore' }
            }
        },
        
        saveKeystoreAs: async (keystoreFileName: string) => {
            try {
                const result = await safeInvoke<string>('save_keystore_as', { fileName: keystoreFileName })
                return JSON.parse(result)
            } catch (error: any) {
                return { success: false, error: error || 'Failed to save keystore' }
            }
        },
        
        updateMinBlockHeight: async (keystoreFileName: string, blockHeight: number | null) => {
            try {
                if (!keystoreFileName) {
                    keystoreFileName = 'default'
                }
                if (blockHeight === null || blockHeight === undefined) {
                    return { success: true }
                }
                const result = await safeInvoke<string>('update_min_block_height', { 
                    fileName: keystoreFileName, 
                    blockHeight 
                })
                return JSON.parse(result)
            } catch (error: any) {
                console.error('[walletApi] Error updating min block height:', error)
                return { success: false, error: error || 'Failed to update min block height' }
            }
        },
        
        buildTransaction: async (payload: any) => {
            try {
                const result = await safeInvoke<string>('build_transaction', { payload })
                return JSON.parse(result)
            } catch (error: any) {
                return { success: false, error: error || 'Failed to build transaction' }
            }
        },
        
        getMinBlockHeight: async (keystoreFileName: string) => {
            try {
                const result = await safeInvoke<string>('get_min_block_height', { fileName: keystoreFileName })
                return JSON.parse(result)
            } catch (error: any) {
                return { success: false, error: error || 'Failed to get min block height' }
            }
        },
        
        saveMinBlockHeight: async (keystoreFileName: string, blockHeight: number) => {
            try {
                const result = await safeInvoke<string>('save_min_block_height', { 
                    fileName: keystoreFileName, 
                    blockHeight 
                })
                return JSON.parse(result)
            } catch (error: any) {
                return { success: false, error: error || 'Failed to save min block height' }
            }
        },
        
        keystoreExists: async (fileName: string) => {
            try {
                const result = await safeInvoke<string>('keystore_exists', { fileName })
                return JSON.parse(result)
            } catch (error: any) {
                return { success: false, error: error || 'Failed to check keystore', exists: false }
            }
        },
        
        checkKeystore: async (fileName?: string) => {
            try {
                const result = await safeInvoke<string>('check_keystore', { fileName })
                return JSON.parse(result)
            } catch (error: any) {
                return { success: false, error: error || 'Failed to check keystore', exists: false }
            }
        },
        
        saveKeystoreDialog: async (keystoreData: string, defaultFileName: string) => {
            try {
                const result = await safeInvoke<string>('save_keystore_dialog', { 
                    keystoreData, 
                    defaultFileName 
                })
                return JSON.parse(result)
            } catch (error: any) {
                return { success: false, error: error || 'Failed to save keystore' }
            }
        },
        
        createKeystore: async (seed: string, password: string) => {
            try {
                const result = await safeInvoke<string>('create_keystore', { seed, password })
                return JSON.parse(result)
            } catch (error: any) {
                return { success: false, error: error || 'Failed to create keystore' }
            }
        },
        
        getKeystorePath: async () => {
            try {
                const result = await safeInvoke<string>('get_keystore_path')
                return JSON.parse(result)
            } catch (error: any) {
                return { success: false, error: error || 'Failed to get keystore path' }
            }
        },

        setRpcEndpoint: async (endpoint: string) => {
            try {
                await safeInvoke('set_rpc_endpoint', { endpoint })
                return { success: true }
            } catch (error: any) {
                return { success: false, error: error?.message || error || 'Failed to set RPC endpoint' }
            }
        },

        getCustomRpcEndpoint: async () => {
            try {
                const result = await safeInvoke<string | null>('get_custom_rpc_endpoint')
                return result || null
            } catch (error: any) {
                return { success: false, error: error || 'Failed to get RPC endpoint' }
            }
        },

        scanUtxosClientSide: async (request: any) => {
            try {
                const result = await safeInvoke<string>('scan_utxos_client_side', { request })
                return JSON.parse(result)
            } catch (error: any) {
                return { success: false, error: error?.message || error || 'Failed to scan UTXOs' }
            }
        },

        clearUtxoCache: async () => {
            try {
                const result = await safeInvoke<string>('clear_utxo_cache')
                return JSON.parse(result)
            } catch (error: any) {
                return { success: false, error: error?.message || error || 'Failed to clear UTXO cache' }
            }
        },

        getPendingBalance: async (request: any) => {
            try {
                const result = await safeInvoke<string>('get_pending_balance', { request })
                return JSON.parse(result)
            } catch (error: any) {
                return { success: false, error: error?.message || error || 'Failed to get pending balance' }
            }
        }
    }
}

function initializeApp() {
    const appElement = document.getElementById('app')
    if (!appElement) {
        sendLogToNative('error', '[Tauri] #app element not found!')
        return
    }

    const app = createApp(App)

    app.use(i18n)
    app.use(createPinia())
    app.use(router)

    const originalAddEventListener = Element.prototype.addEventListener
    Element.prototype.addEventListener = function (type: any, listener: any, options: any) {
        if (type === 'DOMNodeInserted') return
        return originalAddEventListener.call(this, type, listener, options)
    }

    try {
        app.mount('#app')
    } catch (err: any) {
        const msg = `[Tauri] Failed to mount Vue app: ${err?.message || err}`
        sendLogToNative('error', msg)
        throw err
    }
}

initializeApp()
