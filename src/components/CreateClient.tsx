import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { Client } from '../types'

interface CreateClientProps {
  onClientCreated?: (client: Client) => void
  onCancel?: () => void
}

function validateEmail(email: string): boolean {
  const atIdx = email.indexOf('@')
  if (atIdx <= 0) return false
  const afterAt = email.slice(atIdx + 1)
  if (afterAt.includes('@') || afterAt.includes(' ')) return false
  const dotIdx = afterAt.lastIndexOf('.')
  return dotIdx > 0 && dotIdx < afterAt.length - 1
}

export function CreateClient({ onClientCreated, onCancel }: CreateClientProps) {
  const [formData, setFormData] = useState({
    email: '',
    name: '',
    notes: '',
    phone: '',
  })
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState<string | null>()

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!formData.name.trim()) {
      setError('Name is required')
      return
    }
    if (formData.email && !validateEmail(formData.email)) {
      setError('Invalid email format')
      return
    }
    setIsSubmitting(true)
    setError(undefined)

    try {
      const client = await invoke<Client>('create_client', {
        email: formData.email || null,
        name: formData.name,
        notes: formData.notes || null,
        phone: formData.phone || null,
      })
      onClientCreated?.(client)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setIsSubmitting(false)
    }
  }

  const handleChange = (field: keyof typeof formData, value: string) => {
    setFormData((prev) => ({ ...prev, [field]: value }))
    if (field === 'email') setError(undefined)
  }

  return (
    <form
      noValidate
      onSubmit={(e) => {
        e.preventDefault()
        void handleSubmit(e)
      }}
    >
      <div className="flex flex-col gap-sm">
        <div className="flex flex-col gap-xxs">
          <label htmlFor="client-name" className="form-label">
            Name *
          </label>
          <input
            id="client-name"
            type="text"
            className="input"
            value={formData.name}
            onChange={(e) => handleChange('name', e.target.value)}
            placeholder="e.g., Smith Family"
            required
          />
        </div>

        <div className="flex flex-col gap-xxs">
          <label htmlFor="client-email" className="form-label">
            Email <span className="text-optional">(optional)</span>
          </label>
          <input
            id="client-email"
            type="email"
            className="input"
            value={formData.email}
            onChange={(e) => handleChange('email', e.target.value)}
            placeholder="e.g., smith@example.com"
          />
        </div>

        <div className="flex flex-col gap-xxs">
          <label htmlFor="client-phone" className="form-label">
            Phone <span className="text-optional">(optional)</span>
          </label>
          <input
            id="client-phone"
            type="tel"
            className="input"
            value={formData.phone}
            onChange={(e) => handleChange('phone', e.target.value)}
            placeholder="e.g., +1 555 123 4567"
          />
        </div>

        <div className="flex flex-col gap-xxs">
          <label htmlFor="client-notes" className="form-label">
            Notes <span className="text-optional">(optional)</span>
          </label>
          <textarea
            id="client-notes"
            className="input"
            value={formData.notes}
            onChange={(e) => handleChange('notes', e.target.value)}
            placeholder="Any notes about this client"
            rows={3}
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
            backgroundColor: 'var(--color-bg-secondary)',
            borderBottomLeftRadius: 'var(--radius-xl)',
            borderBottomRightRadius: 'var(--radius-xl)',
            borderTop: '1px solid var(--color-border)',
            marginBottom: 'calc(-1 * var(--space-xl))',
            marginLeft: 'calc(-1 * var(--space-xxl))',
            marginRight: 'calc(-1 * var(--space-xxl))',
            marginTop: 'var(--space-lg)',
            paddingBottom: 'var(--space-xl)',
            paddingLeft: 'var(--space-xxl)',
            paddingRight: 'var(--space-xxl)',
            paddingTop: 'var(--space-lg)',
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
              {isSubmitting ? 'Creating...' : 'Create Client'}
            </button>
          </div>
        </div>
      </div>
    </form>
  )
}
