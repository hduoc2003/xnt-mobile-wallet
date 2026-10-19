import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useAccountsStore } from './accountsStore'
import type { Registry } from '@/types'

const registry: Registry = {
  seeds: [
    { id: 'main', keystoreFile: 'k.json', source: 'created', backedUp: true },
    { id: 'imp-1', keystoreFile: 'imp-1.json', source: 'phrase', backedUp: false },
  ],
  accounts: [
    { id: 'main:0', seedId: 'main', index: 0, name: null, hidden: false },
    { id: 'main:2', seedId: 'main', index: 2, name: null, hidden: true },
    { id: 'main:1', seedId: 'main', index: 1, name: 'Savings', hidden: false },
    { id: 'imp-1:0', seedId: 'imp-1', index: 0, name: null, hidden: false },
  ],
  active: 'main:0',
}

describe('accountsStore', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('displayName defaults to Account N', () => {
    const store = useAccountsStore()
    store.setRegistry(registry)
    expect(store.displayName('main:0')).toBe('Account 1')
    expect(store.displayName('main:1')).toBe('Savings')
    expect(store.displayName('main:2')).toBe('Account 3')
    expect(store.displayName('imp-1:0')).toBe('Imported 1 · Account 1')
    expect(() => store.displayName('main:9')).toThrow('Unknown account main:9')
  })

  it('visibleAccounts leaves hidden accounts out', () => {
    const store = useAccountsStore()
    store.setRegistry(registry)
    expect(store.visibleAccounts.map((a) => a.id)).toEqual(['main:0', 'main:1', 'imp-1:0'])
  })
})
