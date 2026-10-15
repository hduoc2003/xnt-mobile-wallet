import { describe, expect, it } from 'vitest'
import { selectSpendableUtxos } from './coinSelection'

const keys = { genKey: 'gen', dctidhKey: 'dctidh' }
const utxo = (amount: string) => ({ amount, keyType: 'Generation' })

describe('selectSpendableUtxos', () => {
  it('stops at maxInputs even when the amount is not covered', () => {
    const utxos = ['1', '1', '1', '1', '1'].map(utxo)
    const selection = selectSpendableUtxos(utxos, 4, 0.1, keys, 2)
    expect(selection.selectedUtxos).toHaveLength(2)
    expect(selection.totalAmount).toBe(2)
  })

  it('takes the largest UTXOs first and stops once amount + fee is covered', () => {
    const utxos = ['0.5', '3', '1'].map(utxo)
    const selection = selectSpendableUtxos(utxos, 3.5, 0.1, keys, 5)
    expect(selection.selectedUtxos.map((u) => u.amount)).toEqual(['3', '1'])
  })
})
