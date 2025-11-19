import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { Project } from '../types'

interface CreateProjectProps {
  onProjectCreated?: (project: Project) => void
  onCancel?: () => void
}

export function CreateProject({ onProjectCreated, onCancel }: CreateProjectProps) {
  const [formData, setFormData] = useState({
    name: '',
    clientName: '',
    date: new Date().toISOString().split('T')[0],
    shootType: '',
  })
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)

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
            <label htmlFor="date" className="font-medium">
              Shoot Date *
            </label>
            <input
              id="date"
              type="date"
              className="input"
              value={formData.date}
              onChange={(e) => handleChange('date', e.target.value)}
              required
            />
          </div>

          <div className="flex flex-col gap-xs">
            <label htmlFor="shootType" className="font-medium">
              Shoot Type *
            </label>
            <input
              id="shootType"
              type="text"
              className="input"
              value={formData.shootType}
              onChange={(e) => handleChange('shootType', e.target.value)}
              placeholder="e.g., Wedding, Portrait, Event"
              required
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
