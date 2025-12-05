import type { DeliveryDestination } from '../types'

export function migrateDeliveryDestinations(parsed: unknown): DeliveryDestination[] {
  if (!Array.isArray(parsed)) {
    throw new Error('Invalid format: not an array')
  }

  return parsed.map((item): DeliveryDestination => {
    if (typeof item === 'object' && item !== null) {
      if ('type' in item) {
        return item as DeliveryDestination
      }
      if ('path' in item && 'id' in item && 'name' in item) {
        return {
          type: 'local',
          id: item.id as string,
          name: item.name as string,
          path: item.path as string,
          enabled: (item.enabled as boolean) ?? true,
          createdAt:
            'createdAt' in item && typeof item.createdAt === 'string'
              ? item.createdAt
              : new Date().toISOString(),
        }
      }
    }
    throw new Error('Invalid destination format')
  })
}
