import { ProjectStatus, type Project } from '../types'
import { formatDateShort } from './formatting'

export function formatProjectInfo(project: Project): string {
  const parts = [
    project.date ? formatDateShort(project.date) : '',
    project.deadline ? `Due ${formatDateShort(project.deadline)}` : '',
  ].filter(Boolean)
  return parts.join(' · ')
}

/**
 * Check if a project deadline is overdue
 * @param deadline - ISO date string to check
 * @returns True if deadline is in the past
 */
export function isOverdue(deadline?: string): boolean {
  if (!deadline) {
    return false
  }
  const today = new Date()
  today.setHours(0, 0, 0, 0)
  const deadlineDate = new Date(deadline)
  deadlineDate.setHours(0, 0, 0, 0)
  return deadlineDate < today
}

const STATUS_ORDER_NEW = 0
const STATUS_ORDER_IMPORTING = 1
const STATUS_ORDER_EDITING = 2
const STATUS_ORDER_DELIVERED = 3
const STATUS_ORDER_ARCHIVED = 4
const STATUS_ORDER_UNKNOWN = 999

/**
 * Get sort order for project status (lower is higher priority)
 * @param status - Project status
 * @returns Numeric sort order
 */
function getStatusOrder(status: ProjectStatus): number {
  switch (status) {
    case ProjectStatus.New: {
      return STATUS_ORDER_NEW
    }
    case ProjectStatus.Importing: {
      return STATUS_ORDER_IMPORTING
    }
    case ProjectStatus.Editing: {
      return STATUS_ORDER_EDITING
    }
    case ProjectStatus.Delivered: {
      return STATUS_ORDER_DELIVERED
    }
    case ProjectStatus.Archived: {
      return STATUS_ORDER_ARCHIVED
    }
    default: {
      return STATUS_ORDER_UNKNOWN
    }
  }
}

/**
 * Sort projects by status (New -> Importing -> Editing -> Delivered -> Archived)
 * then alphabetically by name
 * @param projects - Array of projects to sort
 * @returns Sorted array of projects
 */
export function sortProjectsByStatus(projects: Project[]): Project[] {
  return [...projects].toSorted((a: Project, b: Project) => {
    const statusDiff = getStatusOrder(a.status) - getStatusOrder(b.status)
    if (statusDiff !== 0) {
      return statusDiff
    }
    return a.name.localeCompare(b.name)
  })
}

const STATUS_PRIORITY_IMPORTING = 0
const STATUS_PRIORITY_EDITING = 1
const STATUS_PRIORITY_DELIVERED = 2
const DEFAULT_STATUS_PRIORITY = 999

const STATUS_ORDER_FOR_ACTIVE: Record<string, number> = {
  Delivered: STATUS_PRIORITY_DELIVERED,
  Editing: STATUS_PRIORITY_EDITING,
  Importing: STATUS_PRIORITY_IMPORTING,
}

/**
 * Sort projects by deadline (earliest first), then status, then name.
 * Used in Dashboard and Projects views for active project ordering.
 * @param projects - Array of projects to sort
 * @returns Sorted array of projects
 */
export function sortProjects(projects: Project[]): Project[] {
  return [...projects].toSorted((a: Project, b: Project) => {
    // 1. Sort by deadline (earliest first)
    if (a.deadline && b.deadline) {
      const deadlineDiff = new Date(a.deadline).getTime() - new Date(b.deadline).getTime()
      if (deadlineDiff) {
        return deadlineDiff
      }
    }
    if (a.deadline) {
      return -1
    }
    if (b.deadline) {
      return 1
    }

    // 2. Sort by status
    const statusA = STATUS_ORDER_FOR_ACTIVE[a.status] ?? DEFAULT_STATUS_PRIORITY
    const statusB = STATUS_ORDER_FOR_ACTIVE[b.status] ?? DEFAULT_STATUS_PRIORITY
    if (statusA !== statusB) {
      return statusA - statusB
    }

    // 3. Sort alphabetically by name
    return a.name.localeCompare(b.name)
  })
}
