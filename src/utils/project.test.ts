import { describe, expect, it } from 'vitest'
import { isOverdue } from './project'

describe('isOverdue', () => {
  it('returns false when no deadline provided', () => {
    expect(isOverdue()).toBe(false)
    expect(isOverdue('')).toBe(false)
  })

  it('returns true when deadline is in the past', () => {
    const yesterday = new Date()
    yesterday.setDate(yesterday.getDate() - 1)
    const pastDate = yesterday.toISOString().split('T')[0]

    expect(isOverdue(pastDate)).toBe(true)
  })

  it('returns false when deadline is today', () => {
    const today = new Date().toISOString().split('T')[0]

    expect(isOverdue(today)).toBe(false)
  })

  it('returns false when deadline is in the future', () => {
    const tomorrow = new Date()
    tomorrow.setDate(tomorrow.getDate() + 1)
    const futureDate = tomorrow.toISOString().split('T')[0]

    expect(isOverdue(futureDate)).toBe(false)
  })

  it('handles date strings in ISO format', () => {
    const pastDate = '2020-01-01'
    const futureDate = '2099-12-31'

    expect(isOverdue(pastDate)).toBe(true)
    expect(isOverdue(futureDate)).toBe(false)
  })
})
