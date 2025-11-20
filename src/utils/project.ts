import type { Project } from '../types'

export function formatProjectInfo(project: Project): string {
  const parts = [project.clientName, project.date, project.shootType].filter(Boolean)
  if (project.deadline) {
    parts.push(`Deadline: ${project.deadline}`)
  }
  return parts.join(' · ')
}
