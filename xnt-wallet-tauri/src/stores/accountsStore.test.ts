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

  it('accountWithAddress matches either address of the given accounts, in any case', () => {
    const store = useAccountsStore()
    store.setRegistry(registry)
    const accounts: [string, number][] = [
      ['main:0', 0],
      ['main:1', 1],
    ]
    for (const [id, index] of accounts) {
      store.setKeys(id, {
        index,
        generationKeyHex: `g${index}`,
        dctidhKeyHex: `d${index}`,
        generationAddress: `xntnwt1gen${index}`,
        dctidhAddress: `xntctt1dct${index}`,
      })
    }
    expect(store.accountWithAddress('xntctt1dct1', ['main:0', 'main:1'])).toBe('main:1')
    expect(store.accountWithAddress('  XNTNWT1GEN0 ', ['main:0', 'main:1'])).toBe('main:0')
    expect(store.accountWithAddress('xntctt1dct1', ['main:0'])).toBeUndefined()
    expect(store.accountWithAddress('xntctt1someoneelse', ['main:0', 'main:1'])).toBeUndefined()
    expect(store.accountWithAddress('', ['main:0', 'main:1'])).toBeUndefined()
  })

  it('shortName drops the seed title that displayName adds for imported accounts', () => {
    const store = useAccountsStore()
    store.setRegistry(registry)
    expect(store.shortName('imp-1:0')).toBe('Account 1')
    expect(store.displayName('imp-1:0')).toBe('Imported 1 · Account 1')
    expect(store.shortName('main:1')).toBe('Savings')
    expect(store.defaultName('main:1')).toBe('Account 2')
    expect(store.qualifiedName('main:1')).toBe('Main seed · Savings')
    expect(store.qualifiedName('imp-1:0')).toBe('Imported 1 · Account 1')
  })

  it('visibleAccounts leaves hidden accounts out', () => {
    const store = useAccountsStore()
    store.setRegistry(registry)
    expect(store.visibleAccounts.map((a) => a.id)).toEqual(['main:0', 'main:1', 'imp-1:0'])
  })
})
