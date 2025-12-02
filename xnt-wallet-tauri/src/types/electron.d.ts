// Global type definitions for Electron API
declare global {
    interface Window {
        electron: {
            send: (channel: string, data: any) => void
            receive: (channel: string, func: (...args: any[]) => void) => void
            removeAllListeners: (channel: string) => void
        }
    }
}

export {}
