import type { Project } from '../types'
import { formatDateShort } from './formatting'

export function formatProjectInfo(project: Project): string {
  const parts = [
    project.clientName,
    project.date ? formatDateShort(project.date) : '',
    project.shootType,
  ].filter(Boolean)
  if (project.deadline) {
    parts.push(`Deadline: ${formatDateShort(project.deadline)}`)
  }
  return parts.join(' · ')
}
