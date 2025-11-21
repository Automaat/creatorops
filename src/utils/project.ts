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
 * Get sort order for project status (lower is higher priority)
 */
function getStatusOrder(status: ProjectStatus): number {
  switch (status) {
    case ProjectStatus.New:
      return 0
    case ProjectStatus.Importing:
      return 1
    case ProjectStatus.Editing:
      return 2
    case ProjectStatus.Delivered:
      return 3
    case ProjectStatus.Archived:
      return 4
    default:
      return 999
  }
}

/**
 * Sort projects by status (New -> Importing -> Editing -> Delivered -> Archived)
 * then alphabetically by name
 */
export function sortProjectsByStatus(projects: Project[]): Project[] {
  return [...projects].sort((a, b) => {
    const statusDiff = getStatusOrder(a.status) - getStatusOrder(b.status)
    if (statusDiff !== 0) {
      return statusDiff
    }
    return a.name.localeCompare(b.name)
  })
}
