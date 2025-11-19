import { useEffect, useState } from 'react'
import { open } from '@tauri-apps/plugin-dialog'
import { useTheme } from '../hooks/useTheme'
import type { BackupDestination } from '../types'

export function Settings() {
  const { theme, setTheme } = useTheme()
  const [destinations, setDestinations] = useState<BackupDestination[]>([])
  const [newDestName, setNewDestName] = useState('')

  useEffect(() => {
    loadDestinations()
  }, [])

  function loadDestinations() {
    try {
      const stored = localStorage.getItem('backup_destinations')
      if (stored) {
        setDestinations(JSON.parse(stored))
      }
    } catch (err) {
      console.error('Failed to load destinations:', err)
    }
  }

  function saveDestinations(dests: BackupDestination[]) {
    localStorage.setItem('backup_destinations', JSON.stringify(dests))
    setDestinations(dests)
  }

  async function addDestination() {
    if (!newDestName.trim()) {
      console.warn('Destination name is required')
      return
    }

    try {
      const selected = await open({
        directory: true,
        multiple: false,
      })

      if (selected) {
        const newDest: BackupDestination = {
          id: crypto.randomUUID(),
          name: newDestName.trim(),
          path: selected,
          enabled: true,
          createdAt: new Date().toISOString(),
        }

        saveDestinations([...destinations, newDest])
        setNewDestName('')
      }
    } catch (err) {
      console.error('Failed to add destination:', err)
    }
  }

  function toggleDestination(id: string) {
    const updated = destinations.map((d) => (d.id === id ? { ...d, enabled: !d.enabled } : d))
    saveDestinations(updated)
  }

  function removeDestination(id: string) {
    saveDestinations(destinations.filter((d) => d.id !== id))
  }

  return (
    <>
      <div className="content-header">
        <h1>Settings</h1>
        <p className="text-secondary">Configure CreatorOps preferences</p>
      </div>
      <div className="content-body">
        <div className="flex flex-col gap-xl">
          <section>
            <h2>Appearance</h2>
            <div className="card">
              <div className="flex flex-col gap-md">
                <div>
                  <label className="font-medium">Theme</label>
                  <p className="text-secondary text-sm" style={{ marginBottom: 'var(--space-sm)' }}>
                    Choose how CreatorOps looks
                  </p>
                </div>
                <div className="flex gap-md">
                  <button
                    className={`btn ${theme === 'system' ? 'btn-primary' : 'btn-secondary'}`}
                    onClick={() => setTheme('system')}
                  >
                    System
                  </button>
                  <button
                    className={`btn ${theme === 'light' ? 'btn-primary' : 'btn-secondary'}`}
                    onClick={() => setTheme('light')}
                  >
                    Light
                  </button>
                  <button
                    className={`btn ${theme === 'dark' ? 'btn-primary' : 'btn-secondary'}`}
                    onClick={() => setTheme('dark')}
                  >
                    Dark
                  </button>
                </div>
              </div>
            </div>
          </section>

          <section>
            <h2>Backup Destinations</h2>
            <div className="card">
              <div className="flex flex-col gap-md">
                <p className="text-secondary text-sm">
                  Configure external drives or folders for project backups
                </p>

                {destinations.length > 0 && (
                  <div className="destinations-list">
                    {destinations.map((dest) => (
                      <div key={dest.id} className="destination-item">
                        <div className="destination-info">
                          <div className="flex gap-sm align-center">
                            <input
                              type="checkbox"
                              checked={dest.enabled}
                              onChange={() => toggleDestination(dest.id)}
                            />
                            <span className="font-medium">{dest.name}</span>
                          </div>
                          <p className="text-secondary text-sm">{dest.path}</p>
                        </div>
                        <button
                          onClick={() => removeDestination(dest.id)}
                          className="btn btn-secondary btn-sm"
                        >
                          Remove
                        </button>
                      </div>
                    ))}
                  </div>
                )}

                <div className="add-destination">
                  <input
                    type="text"
                    placeholder="Destination name (e.g., External SSD)"
                    value={newDestName}
                    onChange={(e) => setNewDestName(e.target.value)}
                    className="input"
                    onKeyDown={(e) => e.key === 'Enter' && addDestination()}
                  />
                  <button onClick={addDestination} className="btn btn-primary">
                    Add Destination
                  </button>
                </div>
              </div>
            </div>
          </section>

          <section>
            <h2>Storage Paths</h2>
            <div className="card">
              <div className="flex flex-col gap-md">
                <div className="flex flex-col gap-xs">
                  <label className="font-medium">Default Import Location</label>
                  <p className="text-secondary text-sm">~/CreatorOps/Projects</p>
                </div>
                <div className="flex flex-col gap-xs">
                  <label className="font-medium">Archive Location</label>
                  <p className="text-secondary text-sm">Not configured</p>
                </div>
              </div>
            </div>
          </section>

          <section>
            <h2>Import Settings</h2>
            <div className="card">
              <div className="flex flex-col gap-md">
                <div className="flex flex-col gap-xs">
                  <label className="font-medium">Auto-eject SD cards after import</label>
                  <p className="text-secondary text-sm">Coming soon</p>
                </div>
                <div className="flex flex-col gap-xs">
                  <label className="font-medium">File renaming rules</label>
                  <p className="text-secondary text-sm">Keep original names</p>
                </div>
              </div>
            </div>
          </section>
        </div>
      </div>
    </>
  )
}
