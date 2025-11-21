import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { Project } from '../types'
import { DatePicker } from './DatePicker'

interface CreateProjectProps {
  onProjectCreated?: (project: Project) => void
  onCancel?: () => void
}

const WEEK_IN_MS = 7 * 24 * 60 * 60 * 1000

export function CreateProject({ onProjectCreated, onCancel }: CreateProjectProps) {
  const [formData, setFormData] = useState(() => {
    const today = new Date().toISOString().split('T')[0]
    const oneWeekLater = new Date(Date.now() + WEEK_IN_MS).toISOString().split('T')[0]
    return {
      name: '',
      clientName: '',
      date: today,
      shootType: '',
      deadline: oneWeekLater,
    }
  })
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const shootDate = new Date(formData.date)
    const newDeadline = new Date(shootDate.getTime() + WEEK_IN_MS)
    setFormData((prev) => ({ ...prev, deadline: newDeadline.toISOString().split('T')[0] }))
  }, [formData.date])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setIsSubmitting(true)
    setError(null)

    try {
      const project = await invoke<Project>('create_project', { ...formData })
      onProjectCreated?.(project)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create project')
    } finally {
      setIsSubmitting(false)
    }
  }

  const handleChange = (field: string, value: string) => {
    setFormData((prev) => ({ ...prev, [field]: value }))
  }

  return (
    <div className="card">
      <h2>Create New Project</h2>
      <form onSubmit={handleSubmit}>
        <div className="flex flex-col gap-md" style={{ marginTop: 'var(--space-lg)' }}>
          <div className="flex flex-col gap-xs">
            <label htmlFor="name" className="font-medium">
              Project Name *
            </label>
            <input
              id="name"
              type="text"
              className="input"
              value={formData.name}
              onChange={(e) => handleChange('name', e.target.value)}
              placeholder="e.g., Wedding Portfolio"
              required
            />
          </div>

          <div className="flex flex-col gap-xs">
            <label htmlFor="clientName" className="font-medium">
              Client Name *
            </label>
            <input
              id="clientName"
              type="text"
              className="input"
              value={formData.clientName}
              onChange={(e) => handleChange('clientName', e.target.value)}
              placeholder="e.g., Smith Family"
              required
            />
          </div>

          <div className="flex flex-col gap-xs">
            <DatePicker
              id="date"
              label="Shoot Date"
              value={formData.date}
              onChange={(value) => handleChange('date', value)}
              required
            />
          </div>

          <div className="flex flex-col gap-xs">
            <label htmlFor="shootType" className="font-medium">
              Shoot Type (optional)
            </label>
            <input
              id="shootType"
              type="text"
              className="input"
              value={formData.shootType}
              onChange={(e) => handleChange('shootType', e.target.value)}
              placeholder="e.g., Wedding, Portrait, Event"
            />
          </div>

          <div className="flex flex-col gap-xs">
            <DatePicker
              id="deadline"
              label="Deadline (optional)"
              value={formData.deadline}
              onChange={(value) => handleChange('deadline', value)}
            />
          </div>

          {error && (
            <div className="badge badge-error" style={{ padding: 'var(--space-sm)' }}>
              {error}
            </div>
          )}

          <div className="flex gap-md" style={{ marginTop: 'var(--space-md)' }}>
            <button type="submit" className="btn btn-primary" disabled={isSubmitting}>
              {isSubmitting ? 'Creating...' : 'Create Project'}
            </button>
            {onCancel && (
              <button
                type="button"
                className="btn btn-secondary"
                onClick={onCancel}
                disabled={isSubmitting}
              >
                Cancel
              </button>
            )}
          </div>
        </div>
      </form>
    </div>
  )
}
