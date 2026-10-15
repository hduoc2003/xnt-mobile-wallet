// Endpoints come only from the build's .env files (see .env.example).
const required = (
  name: 'VITE_APP_API' | 'VITE_EXPLORER_URL' | 'VITE_LEVIATHAN_EXPLORER_URL' | 'VITE_PRICE_URL',
): string => {
  const value = import.meta.env[name]
  if (!value) throw new Error(`${name} is not set; copy .env.example to .env`)
  return value
}

export const RPC_URL = required('VITE_APP_API')
export const EXPLORER_URL = required('VITE_EXPLORER_URL')
export const LEVIATHAN_EXPLORER_URL = required('VITE_LEVIATHAN_EXPLORER_URL')
export const PRICE_URL = required('VITE_PRICE_URL')
