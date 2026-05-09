import { describe, expect, it } from 'vitest'
import { formatProjectInfo, isOverdue, sortProjects, sortProjectsByStatus } from './project'
import { ProjectStatus, type Project } from '../types'

describe('formatProjectInfo', () => {
  it('formats project with both date and deadline', () => {
    const project = createMockProject({ date: '2024-01-15', deadline: '2024-06-30' })
    expect(formatProjectInfo(project)).toBe('15 Jan 2024 · Due 30 Jun 2024')
  })

  it('formats project with only date (no deadline)', () => {
    const project = createMockProject({ date: '2024-01-15', deadline: undefined })
    expect(formatProjectInfo(project)).toBe('15 Jan 2024')
  })

  it('formats project with only deadline (empty date)', () => {
    const project = createMockProject({ date: '', deadline: '2024-06-30' })
    expect(formatProjectInfo(project)).toBe('Due 30 Jun 2024')
  })

  it('returns empty string when date and deadline are both absent', () => {
    const project = createMockProject({ date: '', deadline: undefined })
    expect(formatProjectInfo(project)).toBe('')
  })
})

describe('isOverdue', () => {
  it('returns false when no deadline provided', () => {
    expect(isOverdue()).toBe(false)
    expect(isOverdue('')).toBe(false)
  })

  it('returns true when deadline is in the past', () => {
    const yesterday = new Date()
    yesterday.setDate(yesterday.getDate() - 1)
    const [pastDate] = yesterday.toISOString().split('T')

    expect(isOverdue(pastDate)).toBe(true)
  })

  it('returns false when deadline is today', () => {
    const [today] = new Date().toISOString().split('T')

    expect(isOverdue(today)).toBe(false)
  })

  it('returns false when deadline is in the future', () => {
    const tomorrow = new Date()
    tomorrow.setDate(tomorrow.getDate() + 1)
    const [futureDate] = tomorrow.toISOString().split('T')

    expect(isOverdue(futureDate)).toBe(false)
  })

  it('handles date strings in ISO format', () => {
    const pastDate = '2020-01-01'
    const futureDate = '2099-12-31'

    expect(isOverdue(pastDate)).toBe(true)
    expect(isOverdue(futureDate)).toBe(false)
  })

  it('returns false for invalid date string', () => {
    expect(isOverdue('not-a-date')).toBe(false)
  })
})

const createMockProject = (overrides?: Partial<Project>): Project => ({
  id: '1',
  name: 'Test Project',
  clientName: 'Test Client',
  date: '2024-01-15',
  shootType: 'Wedding',
  status: ProjectStatus.Editing,
  folderPath: '/path/to/project',
  createdAt: '2024-01-01T00:00:00Z',
  updatedAt: '2024-01-01T00:00:00Z',
  ...overrides,
})

describe('sortProjects', () => {
  it('sorts projects with deadlines before projects without deadlines', () => {
    const withDeadline = createMockProject({ name: 'A', deadline: '2024-12-31' })
    const withoutDeadline = createMockProject({ name: 'B' })

    const sorted = sortProjects([withoutDeadline, withDeadline])

    expect(sorted[0]).toBe(withDeadline)
    expect(sorted[1]).toBe(withoutDeadline)
  })

  it('sorts project without deadline after project with deadline', () => {
    const withoutDeadline = createMockProject({ name: 'A' })
    const withDeadline = createMockProject({ name: 'B', deadline: '2024-12-31' })

    const sorted = sortProjects([withoutDeadline, withDeadline])

    expect(sorted[0]).toBe(withDeadline)
    expect(sorted[1]).toBe(withoutDeadline)
  })

  it('sorts by earliest deadline when both have deadlines', () => {
    const laterDeadline = createMockProject({ name: 'A', deadline: '2024-12-31' })
    const earlierDeadline = createMockProject({ name: 'B', deadline: '2024-06-15' })

    const sorted = sortProjects([laterDeadline, earlierDeadline])

    expect(sorted[0]).toBe(earlierDeadline)
    expect(sorted[1]).toBe(laterDeadline)
  })

  it('sorts by status when neither have deadlines', () => {
    const delivered = createMockProject({ name: 'A', status: ProjectStatus.Delivered })
    const importing = createMockProject({ name: 'B', status: ProjectStatus.Importing })

    const sorted = sortProjects([delivered, importing])

    expect(sorted[0]).toBe(importing)
    expect(sorted[1]).toBe(delivered)
  })

  it('sorts by name when status and deadline are equal', () => {
    const projectB = createMockProject({ name: 'B', status: ProjectStatus.Editing })
    const projectA = createMockProject({ name: 'A', status: ProjectStatus.Editing })

    const sorted = sortProjects([projectB, projectA])

    expect(sorted[0]).toBe(projectA)
    expect(sorted[1]).toBe(projectB)
  })

  it('sorts by name when same deadline', () => {
    const projectB = createMockProject({
      name: 'B',
      deadline: '2024-12-31',
      status: ProjectStatus.Editing,
    })
    const projectA = createMockProject({
      name: 'A',
      deadline: '2024-12-31',
      status: ProjectStatus.Editing,
    })

    const sorted = sortProjects([projectB, projectA])

    expect(sorted[0]).toBe(projectA)
    expect(sorted[1]).toBe(projectB)
  })

  it('sorts by status when deadlines are equal', () => {
    const editing = createMockProject({
      name: 'A',
      deadline: '2024-12-31',
      status: ProjectStatus.Editing,
    })
    const importing = createMockProject({
      name: 'B',
      deadline: '2024-12-31',
      status: ProjectStatus.Importing,
    })

    const sorted = sortProjects([editing, importing])

    expect(sorted[0]).toBe(importing)
    expect(sorted[1]).toBe(editing)
  })

  it('sorts New and Archived projects after active statuses when no deadlines', () => {
    const newProject = createMockProject({ name: 'A', status: ProjectStatus.New })
    const editing = createMockProject({ name: 'B', status: ProjectStatus.Editing })

    const sorted = sortProjects([newProject, editing])

    expect(sorted[0]).toBe(editing)
    expect(sorted[1]).toBe(newProject)
  })

  it('returns empty array for empty input', () => {
    expect(sortProjects([])).toEqual([])
  })

  it('returns single project unchanged', () => {
    const project = createMockProject({ name: 'Solo' })
    expect(sortProjects([project])).toEqual([project])
  })

  it('does not mutate the original array', () => {
    const projectB = createMockProject({ name: 'B' })
    const projectA = createMockProject({ name: 'A' })
    const original = [projectB, projectA]
    sortProjects(original)
    expect(original[0]).toBe(projectB)
    expect(original[1]).toBe(projectA)
  })
})

describe('sortProjectsByStatus', () => {
  it('sorts projects by status order', () => {
    const archived = createMockProject({ name: 'A', status: ProjectStatus.Archived })
    const newProject = createMockProject({ name: 'B', status: ProjectStatus.New })
    const editing = createMockProject({ name: 'C', status: ProjectStatus.Editing })

    const sorted = sortProjectsByStatus([archived, editing, newProject])

    expect(sorted[0]).toBe(newProject)
    expect(sorted[1]).toBe(editing)
    expect(sorted[2]).toBe(archived)
  })

  it('sorts alphabetically when status is the same', () => {
    const projectB = createMockProject({ name: 'B', status: ProjectStatus.Editing })
    const projectA = createMockProject({ name: 'A', status: ProjectStatus.Editing })

    const sorted = sortProjectsByStatus([projectB, projectA])

    expect(sorted[0]).toBe(projectA)
    expect(sorted[1]).toBe(projectB)
  })

  it('orders all five statuses correctly', () => {
    const archived = createMockProject({ name: 'E', status: ProjectStatus.Archived })
    const delivered = createMockProject({ name: 'D', status: ProjectStatus.Delivered })
    const editing = createMockProject({ name: 'C', status: ProjectStatus.Editing })
    const importing = createMockProject({ name: 'B', status: ProjectStatus.Importing })
    const newProject = createMockProject({ name: 'A', status: ProjectStatus.New })

    const sorted = sortProjectsByStatus([archived, delivered, editing, importing, newProject])

    expect(sorted[0]).toBe(newProject)
    expect(sorted[1]).toBe(importing)
    expect(sorted[2]).toBe(editing)
    expect(sorted[3]).toBe(delivered)
    expect(sorted[4]).toBe(archived)
  })

  it('returns empty array for empty input', () => {
    expect(sortProjectsByStatus([])).toEqual([])
  })

  it('does not mutate the original array', () => {
    const projectB = createMockProject({ name: 'B', status: ProjectStatus.Archived })
    const projectA = createMockProject({ name: 'A', status: ProjectStatus.New })
    const original = [projectB, projectA]
    sortProjectsByStatus(original)
    expect(original[0]).toBe(projectB)
    expect(original[1]).toBe(projectA)
  })
})
