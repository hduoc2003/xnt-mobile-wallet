import type { BridgeDepositRecord } from '@/types'

export type BadgeVariant = 'default' | 'secondary' | 'destructive' | 'outline'

export const REFUSAL_REASON = {
  invalid: 'invalid deposit',
  below_mint_fee: 'amount below the mint fee',
  no_snapshot: 'no bridge snapshot yet',
} as const

export const RELAYER_STATE_LABEL = {
  minted: 'minted',
  submitted: 'mint submitted',
  refused: 'refused',
  waiting: 'waiting for a mintable block',
} as const

/** XNT blocks until the relayer may mint this deposit; `undefined` when not known or not behind. */
export const blocksUntilMintable = (record: BridgeDepositRecord): number | undefined => {
  const { status, relayer } = record
  if (status.state !== 'confirmed' || status.blockHeight === null) return undefined
  if (!relayer || relayer.mintableUpTo === null) return undefined
  const behind = status.blockHeight - relayer.mintableUpTo
  return behind > 0 ? behind : undefined
}

export const blocksText = (n: number) => `${n} more XNT block${n === 1 ? '' : 's'}`

export const bridgeDepositBadge = (
  record: BridgeDepositRecord,
): { label: string; variant: BadgeVariant } => {
  switch (record.status.state) {
    case 'submitted':
      return { label: 'Submitted', variant: 'outline' }
    case 'confirmed':
      return { label: 'Confirmed', variant: 'secondary' }
    case 'minted':
      return { label: 'WXNT minted', variant: 'default' }
    case 'dropped':
      return { label: 'Dropped', variant: 'destructive' }
  }
}

export const bridgeDepositDetail = (record: BridgeDepositRecord): string => {
  const { status, relayer, relayerError } = record
  switch (status.state) {
    case 'submitted':
      return 'Waiting for an XNT block'
    case 'dropped':
      return 'Left the mempool unmined; the coins come back after the next balance scan'
    case 'minted':
      return status.blockHeight === null
        ? 'Minted on Leviathan'
        : `Locked in XNT block ${status.blockHeight}, minted on Leviathan`
    case 'confirmed': {
      const block = status.blockHeight === null ? 'Confirmed' : `In XNT block ${status.blockHeight}`
      if (relayerError) return `${block}; can't check mint status right now`
      if (!relayer) return block
      if (relayer.paused) return `${block}; bridge paused`
      switch (relayer.status) {
        case 'refused':
          return `${block}; relayer refused${relayer.reason ? `: ${REFUSAL_REASON[relayer.reason]}` : ''}`
        case 'submitted':
        case 'minted':
          return `${block}; minting WXNT`
        case 'waiting': {
          const behind = blocksUntilMintable(record)
          return behind === undefined
            ? `${block}; waiting for the relayer`
            : `${block}; mintable after ${blocksText(behind)}`
        }
      }
    }
  }
}

/** Not minted, not dropped, and not refused for good (an invalid deposit never mints). */
export const isBridgeDepositInProgress = (record: BridgeDepositRecord): boolean => {
  if (record.status.state === 'minted' || record.status.state === 'dropped') return false
  return !(record.relayer?.status === 'refused' && record.relayer.reason === 'invalid')
}
