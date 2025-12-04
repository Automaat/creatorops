import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { Project } from '../types'
import { DatePicker } from './DatePicker'

interface CreateProjectProps {
  onProjectCreated?: (project: Project) => void
  onCancel?: () => void
}

export function CreateProject({ onProjectCreated, onCancel }: CreateProjectProps) {
  const [formData, setFormData] = useState(() => {
    const today = new Date().toISOString().split('T')[0]
    return {
      clientName: '', date: today, deadline: '', name: '', shootType: '',
    }
  })
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState<string | null>()

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setIsSubmitting(true)
    setError(undefined)

    try {
      const project = await invoke<Project>('create_project', { ...formData })
      onProjectCreated?.(project)
    } catch (error) {
      setError(error instanceof Error ? error.message : 'Failed to create project')
    } finally {
      setIsSubmitting(false)
    }
  }

  const handleChange = <K extends keyof typeof formData>(field: K, value: (typeof formData)[K]) => {
    setFormData((prev) => ({ ...prev, [field]: value }))
  }

  return (
    <form onSubmit={(e) => { e.preventDefault(); void handleSubmit(e); }}>
      <div className="flex flex-col gap-sm">
        <div className="flex flex-col gap-xxs">
          <label htmlFor="name" className="form-label">
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

        <div className="flex flex-col gap-xxs">
          <label htmlFor="clientName" className="form-label">
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

        <div className="flex flex-col gap-xxs">
          <DatePicker
            id="date"
            label="Shoot Date"
            value={formData.date}
            onChange={(value) => handleChange('date', value)}
            required
          />
        </div>

        <hr className="form-section-separator" />

        <div className="flex flex-col gap-xxs">
          <label htmlFor="shootType" className="form-label">
            Shoot Type <span className="text-optional">(optional)</span>
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

        <div className="flex flex-col gap-xxs">
          <DatePicker
            id="deadline"
            label={
              <>
                Deadline <span className="text-optional">(optional)</span>
              </>
            }
            value={formData.deadline}
            onChange={(value) => handleChange('deadline', value)}
          />
        </div>

        {error && (
          <div className="badge badge-error" style={{ padding: 'var(--space-sm)' }}>
            {error}
          </div>
        )}

        <div
          className="form-footer"
          style={{
            backgroundColor: 'var(--color-bg-secondary)', borderBottomLeftRadius: 'var(--radius-xl)', borderBottomRightRadius: 'var(--radius-xl)', borderTop: '1px solid var(--color-border)', marginBottom: 'calc(-1 * var(--space-xl))', marginLeft: 'calc(-1 * var(--space-xxl))', marginRight: 'calc(-1 * var(--space-xxl))', marginTop: 'var(--space-lg)', paddingBottom: 'var(--space-xl)', paddingLeft: 'var(--space-xxl)', paddingRight: 'var(--space-xxl)', paddingTop: 'var(--space-lg)',
          }}
        >
          <div className="flex gap-md" style={{ justifyContent: 'flex-end' }}>
            {onCancel && (
              <button
                type="button"
                className="btn btn-ghost"
                onClick={onCancel}
                disabled={isSubmitting}
              >
                Cancel
              </button>
            )}
            <button type="submit" className="btn btn-primary" disabled={isSubmitting}>
              {isSubmitting ? 'Creating...' : 'Create Project'}
            </button>
          </div>
        </div>
      </div>
    </form>
  )
}
