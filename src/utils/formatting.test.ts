import { describe, expect, it } from 'vitest'
import {
  formatBytes,
  formatSpeed,
  formatETA,
  formatDate,
  formatDateShort,
  formatDisplayDate,
  MONTH_NAMES_SHORT,
  MONTH_NAMES_FULL,
} from './formatting'

describe('formatBytes', () => {
  it('formats 0 bytes', () => {
    expect(formatBytes(0)).toBe('0 B')
  })

  it('formats bytes', () => {
    expect(formatBytes(500)).toBe('500.00 B')
  })

  it('formats kilobytes', () => {
    expect(formatBytes(1024)).toBe('1.00 KB')
    expect(formatBytes(1536)).toBe('1.50 KB')
  })

  it('formats megabytes', () => {
    expect(formatBytes(1048576)).toBe('1.00 MB')
    expect(formatBytes(2621440)).toBe('2.50 MB')
  })

  it('formats gigabytes', () => {
    expect(formatBytes(1073741824)).toBe('1.00 GB')
    expect(formatBytes(5368709120)).toBe('5.00 GB')
  })

  it('formats terabytes', () => {
    expect(formatBytes(1099511627776)).toBe('1.00 TB')
  })

  it('handles negative numbers', () => {
    expect(formatBytes(-100)).toBe('0 B')
  })

  it('handles non-finite numbers', () => {
    expect(formatBytes(Number.POSITIVE_INFINITY)).toBe('0 B')
    expect(formatBytes(Number.NEGATIVE_INFINITY)).toBe('0 B')
    expect(formatBytes(Number.NaN)).toBe('0 B')
  })
})

describe('formatSpeed', () => {
  it('formats speed in bytes per second', () => {
    expect(formatSpeed(1024)).toBe('1.00 KB/s')
  })

  it('formats speed in megabytes per second', () => {
    expect(formatSpeed(1048576)).toBe('1.00 MB/s')
  })

  it('formats zero speed', () => {
    expect(formatSpeed(0)).toBe('0 B/s')
  })
})

describe('formatETA', () => {
  it('returns -- for 0 seconds', () => {
    expect(formatETA(0)).toBe('--')
  })

  it('formats seconds only', () => {
    expect(formatETA(45)).toBe('45s')
  })

  it('formats minutes and seconds', () => {
    expect(formatETA(90)).toBe('1m 30s')
    expect(formatETA(125)).toBe('2m 5s')
  })

  it('formats hours and minutes', () => {
    expect(formatETA(3600)).toBe('1h 0m')
    expect(formatETA(3661)).toBe('1h 1m')
    expect(formatETA(7200)).toBe('2h 0m')
  })

  it('formats hours, minutes, seconds', () => {
    expect(formatETA(3725)).toBe('1h 2m')
  })
})

describe('formatDate', () => {
  it('formats unix timestamp', () => {
    const result = formatDate('1704067200')
    expect(result).toContain('Jan')
    expect(result).toContain('2024')
  })

  it('returns original string for invalid timestamp', () => {
    expect(formatDate('invalid')).toBe('invalid')
  })

  it('returns original string for non-numeric input', () => {
    expect(formatDate('not-a-number')).toBe('not-a-number')
  })

  it('handles empty string', () => {
    expect(formatDate('')).toBe('')
  })
})

describe('formatDateShort', () => {
  it('formats valid date string', () => {
    const result = formatDateShort('2025-01-15')
    expect(result).toBe('15 Jan 2025')
  })

  it('formats valid date object', () => {
    const result = formatDateShort('2025-12-25')
    expect(result).toBe('25 Dec 2025')
  })

  it('returns original string for invalid date', () => {
    expect(formatDateShort('invalid')).toBe('invalid')
  })

  it('returns original string for empty string', () => {
    expect(formatDateShort('')).toBe('')
  })

  it('formats date with single digit day', () => {
    const result = formatDateShort('2025-03-05')
    expect(result).toBe('5 Mar 2025')
  })
})

describe('formatDisplayDate', () => {
  it('formats date string', () => {
    const result = formatDisplayDate('2025-01-15')
    expect(result).toBe('Jan 15, 2025')
  })

  it('formats date object', () => {
    const date = new Date('2025-03-20')
    const result = formatDisplayDate(date)
    expect(result).toBe('Mar 20, 2025')
  })

  it('formats all months correctly', () => {
    const months = [
      { date: '2025-01-10', expected: 'Jan 10, 2025' },
      { date: '2025-02-10', expected: 'Feb 10, 2025' },
      { date: '2025-03-10', expected: 'Mar 10, 2025' },
      { date: '2025-04-10', expected: 'Apr 10, 2025' },
      { date: '2025-05-10', expected: 'May 10, 2025' },
      { date: '2025-06-10', expected: 'Jun 10, 2025' },
      { date: '2025-07-10', expected: 'Jul 10, 2025' },
      { date: '2025-08-10', expected: 'Aug 10, 2025' },
      { date: '2025-09-10', expected: 'Sep 10, 2025' },
      { date: '2025-10-10', expected: 'Oct 10, 2025' },
      { date: '2025-11-10', expected: 'Nov 10, 2025' },
      { date: '2025-12-10', expected: 'Dec 10, 2025' },
    ]

    for (const { date, expected } of months) {
      expect(formatDisplayDate(date)).toBe(expected)
    }
  })
})

describe('month constants', () => {
  it('exports short month names', () => {
    expect(MONTH_NAMES_SHORT).toHaveLength(12)
    expect(MONTH_NAMES_SHORT[0]).toBe('Jan')
    expect(MONTH_NAMES_SHORT[11]).toBe('Dec')
  })

  it('exports full month names', () => {
    expect(MONTH_NAMES_FULL).toHaveLength(12)
    expect(MONTH_NAMES_FULL[0]).toBe('January')
    expect(MONTH_NAMES_FULL[11]).toBe('December')
  })
})
