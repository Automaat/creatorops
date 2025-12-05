import { describe, expect, it } from 'vitest'
import { migrateDeliveryDestinations } from './deliveryDestinations'
import type { DeliveryDestination } from '../types'

describe('migrateDeliveryDestinations', () => {
  it('throws error when input is not an array', () => {
    expect(() => migrateDeliveryDestinations('not an array')).toThrow(
      'Invalid format: not an array'
    )
    expect(() => migrateDeliveryDestinations(null)).toThrow('Invalid format: not an array')
    expect(() => migrateDeliveryDestinations(undefined)).toThrow('Invalid format: not an array')
    expect(() => migrateDeliveryDestinations({ foo: 'bar' })).toThrow(
      'Invalid format: not an array'
    )
  })

  it('returns destinations with type field unchanged', () => {
    const destinations: DeliveryDestination[] = [
      {
        type: 'local',
        id: 'dest-1',
        name: 'Local Folder',
        path: '/path/to/folder',
        enabled: true,
        createdAt: '2024-01-01T00:00:00Z',
      },
      {
        type: 'google-drive',
        id: 'dest-2',
        name: 'Google Drive',
        accountId: 'account-1',
        folderId: 'folder-1',
        enabled: true,
        createdAt: '2024-01-02T00:00:00Z',
      },
    ]

    const result = migrateDeliveryDestinations(destinations)

    expect(result).toEqual(destinations)
  })

  it('migrates old format without type field to local destination', () => {
    const oldFormat = [
      {
        id: 'dest-1',
        name: 'Client Portal',
        path: '/Volumes/ClientDelivery',
        enabled: true,
        createdAt: '2024-01-01T00:00:00Z',
      },
    ]

    const result = migrateDeliveryDestinations(oldFormat)

    expect(result).toEqual([
      {
        type: 'local',
        id: 'dest-1',
        name: 'Client Portal',
        path: '/Volumes/ClientDelivery',
        enabled: true,
        createdAt: '2024-01-01T00:00:00Z',
      },
    ])
  })

  it('migrates old format without createdAt field', () => {
    const oldFormat = [
      {
        id: 'dest-1',
        name: 'Client Portal',
        path: '/Volumes/ClientDelivery',
        enabled: true,
      },
    ]

    const result = migrateDeliveryDestinations(oldFormat)

    expect(result).toHaveLength(1)
    expect(result[0]).toMatchObject({
      type: 'local',
      id: 'dest-1',
      name: 'Client Portal',
      path: '/Volumes/ClientDelivery',
      enabled: true,
    })
    expect(result[0].createdAt).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/)
  })

  it('migrates old format with enabled defaulting to true', () => {
    const oldFormat = [
      {
        id: 'dest-1',
        name: 'Client Portal',
        path: '/Volumes/ClientDelivery',
      },
    ]

    const result = migrateDeliveryDestinations(oldFormat)

    expect(result[0].enabled).toBe(true)
  })

  it('throws error for invalid destination format', () => {
    expect(() => migrateDeliveryDestinations([{ foo: 'bar' }])).toThrow(
      'Invalid destination format'
    )
    expect(() => migrateDeliveryDestinations([null])).toThrow('Invalid destination format')
    expect(() => migrateDeliveryDestinations(['string'])).toThrow('Invalid destination format')
    expect(() => migrateDeliveryDestinations([123])).toThrow('Invalid destination format')
  })

  it('throws error when destination is missing required fields', () => {
    expect(() => migrateDeliveryDestinations([{ id: 'dest-1', name: 'Test' }])).toThrow(
      'Invalid destination format'
    )
    expect(() => migrateDeliveryDestinations([{ id: 'dest-1', path: '/path' }])).toThrow(
      'Invalid destination format'
    )
    expect(() => migrateDeliveryDestinations([{ name: 'Test', path: '/path' }])).toThrow(
      'Invalid destination format'
    )
  })

  it('handles mixed array of old and new formats', () => {
    const mixed = [
      {
        type: 'local',
        id: 'dest-1',
        name: 'New Format',
        path: '/new/path',
        enabled: true,
        createdAt: '2024-01-01T00:00:00Z',
      },
      {
        id: 'dest-2',
        name: 'Old Format',
        path: '/old/path',
        enabled: false,
        createdAt: '2024-01-02T00:00:00Z',
      },
    ]

    const result = migrateDeliveryDestinations(mixed)

    expect(result).toHaveLength(2)
    expect(result[0]).toEqual(mixed[0])
    expect(result[1]).toEqual({
      type: 'local',
      id: 'dest-2',
      name: 'Old Format',
      path: '/old/path',
      enabled: false,
      createdAt: '2024-01-02T00:00:00Z',
    })
  })

  it('handles empty array', () => {
    const result = migrateDeliveryDestinations([])
    expect(result).toEqual([])
  })
})
