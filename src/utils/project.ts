import type { Project } from '../types'
import { ProjectStatus } from '../types'
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
 */
export function isOverdue(deadline?: string): boolean {
  if (!deadline) {return false}
  const today = new Date()
  today.setHours(0, 0, 0, 0)
  const deadlineDate = new Date(deadline)
  deadlineDate.setHours(0, 0, 0, 0)
  return deadlineDate < today
}

/**
 * Get sort order for project status (lower is higher priority)
 */
function getStatusOrder(status: ProjectStatus): number {
  switch (status) {
    case ProjectStatus.New: {
      return 0
    }
    case ProjectStatus.Importing: {
      return 1
    }
    case ProjectStatus.Editing: {
      return 2
    }
    case ProjectStatus.Delivered: {
      return 3
    }
    case ProjectStatus.Archived: {
      return 4
    }
    default: {
      return 999
    }
  }
}

/**
 * Sort projects by status (New -> Importing -> Editing -> Delivered -> Archived)
 * then alphabetically by name
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

const STATUS_ORDER_FOR_ACTIVE: Record<string, number> = {
  Delivered: 2, Editing: 1, Importing: 0,
}

/**
 * Sort projects by deadline (earliest first), then status, then name.
 * Used in Dashboard and Projects views for active project ordering.
 */
export function sortProjects(projects: Project[]): Project[] {
  return [...projects].toSorted((a: Project, b: Project) => {
    // 1. Sort by deadline (earliest first)
    if (a.deadline && b.deadline) {
      const deadlineDiff = new Date(a.deadline).getTime() - new Date(b.deadline).getTime()
      if (deadlineDiff) {return deadlineDiff}
    }
    if (a.deadline) {return -1}
    if (b.deadline) {return 1}

    // 2. Sort by status
    const statusA = STATUS_ORDER_FOR_ACTIVE[a.status] ?? 999
    const statusB = STATUS_ORDER_FOR_ACTIVE[b.status] ?? 999
    if (statusA !== statusB) {return statusA - statusB}

    // 3. Sort alphabetically by name
    return a.name.localeCompare(b.name)
  })
}
