import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

vi.mock('@/api/neptuneApi', () => ({
  callJsonRpc: vi.fn(async () => ({ result: { header: { timestamp: 0 } } })),
}))

import { useTransaction } from './useTransaction'
import type { ReceivedEntry } from '@/types'
import type { NormalizedUtxo } from '@/utils'

const utxo = (utxoId: string): NormalizedUtxo => ({
  utxoId,
  utxoDigest: utxoId.split(':')[0],
  amount: '1',
  blockHeight: 5,
  additionRecord: '0x0',
  isTimelocked: false,
  releaseDateMs: null,
  keyType: 'Generation',
  isPending: false,
  isLegacyDctidh: false,
})

describe('syncReceived', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    const items = new Map<string, string>()
    vi.stubGlobal('window', {
      localStorage: {
        getItem: (key: string) => items.get(key) ?? null,
        setItem: (key: string, value: string) => void items.set(key, value),
      },
      walletApi: { bridgeBurnSerial: vi.fn(async () => null) },
    })
  })

  it('logs identical payments once per UTXO', async () => {
    const { syncReceived } = useTransaction()
    const first = await syncReceived([utxo('c:3'), utxo('c:4')], new Set(), 'main:0')
    expect(first.received.map((r) => r.utxoId)).toEqual(['c:3', 'c:4'])
    const again = await syncReceived([utxo('c:3'), utxo('c:4')], new Set(), 'main:0')
    expect(again.received).toHaveLength(2)
  })

  it('does not relog a UTXO logged before utxo ids', async () => {
    const legacy: ReceivedEntry = { commitment: 'c', amount: '1', blockHeight: 5, time: 'x' }
    window.localStorage.setItem('xnt_received_history:main:0', JSON.stringify([legacy]))
    const { syncReceived } = useTransaction()
    const { received } = await syncReceived([utxo('c:3')], new Set(), 'main:0')
    expect(received).toEqual([legacy])
  })
})
