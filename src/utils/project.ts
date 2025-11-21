import type { Project } from '../types'
import { formatDateShort } from './formatting'

export function formatProjectInfo(project: Project): string {
  const parts = [
    project.date ? formatDateShort(project.date) : '',
    project.deadline ? `Due ${formatDateShort(project.deadline)}` : '',
  ].filter(Boolean)
  return parts.join(' · ')
}
