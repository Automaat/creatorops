import { useEffect, useState } from 'react'
import { open } from '@tauri-apps/plugin-dialog'
import { useTheme } from '../hooks/useTheme'
import { useNotification } from '../hooks/useNotification'
import type { BackupDestination, DeliveryDestination } from '../types'

const DEFAULT_FOLDER_TEMPLATE = '{YYYY}-{MM}-{DD}_{ClientName}_{Type}'
const DEFAULT_FILE_TEMPLATE = '{original}'

export function Settings() {
  const { theme, setTheme } = useTheme()
  const { error: showError } = useNotification()
  const [destinations, setDestinations] = useState<BackupDestination[]>([])
  const [newDestName, setNewDestName] = useState('')
  const [deliveryDestinations, setDeliveryDestinations] = useState<DeliveryDestination[]>([])
  const [newDeliveryDestName, setNewDeliveryDestName] = useState('')
  const [archiveLocation, setArchiveLocation] = useState('')
  const [folderTemplate, setFolderTemplate] = useState(DEFAULT_FOLDER_TEMPLATE)
  const [fileRenameTemplate, setFileRenameTemplate] = useState(DEFAULT_FILE_TEMPLATE)
  const [autoEject, setAutoEject] = useState(false)

  useEffect(() => {
    loadDestinations()
    loadDeliveryDestinations()
    loadArchiveLocation()
    loadTemplates()
    loadAutoEject()
    // eslint-disable-next-line react-hooks/exhaustive-deps
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

  function loadDeliveryDestinations() {
    try {
      const stored = localStorage.getItem('delivery_destinations')
      if (stored) {
        setDeliveryDestinations(JSON.parse(stored))
      }
    } catch (err) {
      console.error('Failed to load delivery destinations:', err)
    }
  }

  function loadArchiveLocation() {
    try {
      const stored = localStorage.getItem('archive_location')
      if (stored) {
        setArchiveLocation(stored)
      }
    } catch (err) {
      console.error('Failed to load archive location:', err)
    }
  }

  function saveDeliveryDestinations(dests: DeliveryDestination[]) {
    localStorage.setItem('delivery_destinations', JSON.stringify(dests))
    setDeliveryDestinations(dests)
  }

  async function addDeliveryDestination() {
    if (!newDeliveryDestName.trim()) {
      return
    }

    try {
      const selected = await open({
        directory: true,
        multiple: false,
      })

      if (selected) {
        const newDest: DeliveryDestination = {
          id: crypto.randomUUID(),
          name: newDeliveryDestName.trim(),
          path: selected,
          enabled: true,
          createdAt: new Date().toISOString(),
        }

        saveDeliveryDestinations([...deliveryDestinations, newDest])
        setNewDeliveryDestName('')
      }
    } catch (err) {
      console.error('Failed to add delivery destination:', err)
    }
  }

  function toggleDeliveryDestination(id: string) {
    const updated = deliveryDestinations.map((d) =>
      d.id === id ? { ...d, enabled: !d.enabled } : d
    )
    saveDeliveryDestinations(updated)
  }

  function removeDeliveryDestination(id: string) {
    saveDeliveryDestinations(deliveryDestinations.filter((d) => d.id !== id))
  }

  async function selectArchiveLocation() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
      })

      if (selected) {
        localStorage.setItem('archive_location', selected)
        setArchiveLocation(selected)
      }
    } catch (err) {
      console.error('Failed to select archive location:', err)
    }
  }

  function clearArchiveLocation() {
    localStorage.removeItem('archive_location')
    setArchiveLocation('')
  }

  function loadTemplates() {
    try {
      const storedFolderTemplate = localStorage.getItem('folder_template')
      const storedFileTemplate = localStorage.getItem('file_rename_template')
      if (storedFolderTemplate) setFolderTemplate(storedFolderTemplate)
      if (storedFileTemplate) setFileRenameTemplate(storedFileTemplate)
    } catch (err) {
      console.error('Failed to load templates:', err)
      showError('Failed to load template settings')
    }
  }

  function loadAutoEject() {
    try {
      const stored = localStorage.getItem('auto_eject')
      if (stored) setAutoEject(stored === 'true')
    } catch (err) {
      console.error('Failed to load auto-eject setting:', err)
      showError('Failed to load auto-eject setting')
    }
  }

  function saveFolderTemplate(template: string) {
    localStorage.setItem('folder_template', template)
    setFolderTemplate(template)
  }

  function saveFileRenameTemplate(template: string) {
    localStorage.setItem('file_rename_template', template)
    setFileRenameTemplate(template)
  }

  function toggleAutoEject() {
    const newValue = !autoEject
    localStorage.setItem('auto_eject', newValue.toString())
    setAutoEject(newValue)
  }

  function resetTemplates() {
    saveFolderTemplate(DEFAULT_FOLDER_TEMPLATE)
    saveFileRenameTemplate(DEFAULT_FILE_TEMPLATE)
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
            <h2>Delivery Destinations</h2>
            <div className="card">
              <div className="flex flex-col gap-md">
                <p className="text-secondary text-sm">
                  Configure folders for client delivery exports
                </p>

                {deliveryDestinations.length > 0 && (
                  <div className="destinations-list">
                    {deliveryDestinations.map((dest) => (
                      <div key={dest.id} className="destination-item">
                        <div className="destination-info">
                          <div className="flex gap-sm align-center">
                            <input
                              type="checkbox"
                              checked={dest.enabled}
                              onChange={() => toggleDeliveryDestination(dest.id)}
                            />
                            <span className="font-medium">{dest.name}</span>
                          </div>
                          <p className="text-secondary text-sm">{dest.path}</p>
                        </div>
                        <button
                          onClick={() => removeDeliveryDestination(dest.id)}
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
                    placeholder="Destination name (e.g., Client Delivery Folder)"
                    value={newDeliveryDestName}
                    onChange={(e) => setNewDeliveryDestName(e.target.value)}
                    className="input"
                    onKeyDown={(e) => e.key === 'Enter' && addDeliveryDestination()}
                  />
                  <button onClick={addDeliveryDestination} className="btn btn-primary">
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
                <div className="flex flex-col gap-md">
                  <label className="font-medium">Archive Location</label>
                  <p className="text-secondary text-sm">{archiveLocation || 'Not configured'}</p>
                  <div className="flex gap-sm">
                    <button onClick={selectArchiveLocation} className="btn btn-primary">
                      {archiveLocation ? 'Change Location' : 'Select Location'}
                    </button>
                    {archiveLocation && (
                      <button onClick={clearArchiveLocation} className="btn btn-secondary">
                        Clear
                      </button>
                    )}
                  </div>
                </div>
              </div>
            </div>
          </section>

          <section>
            <h2>Folder Templates</h2>
            <div className="card">
              <div className="flex flex-col gap-md">
                <div className="flex flex-col gap-xs">
                  <label className="font-medium">Project Folder Template</label>
                  <p className="text-secondary text-sm">
                    Available variables: {'{YYYY}'}, {'{MM}'}, {'{DD}'}, {'{ClientName}'},{' '}
                    {'{Type}'}
                  </p>
                  <input
                    type="text"
                    className="input"
                    value={folderTemplate}
                    onChange={(e) => saveFolderTemplate(e.target.value)}
                    placeholder="{YYYY}-{MM}-{DD}_{ClientName}_{Type}"
                  />
                  <p className="text-secondary text-sm">Preview: 2024-01-15_JohnDoe_Wedding</p>
                </div>
                <div className="flex flex-col gap-xs">
                  <label className="font-medium">File Rename Template</label>
                  <p className="text-secondary text-sm">
                    Available variables: {'{original}'}, {'{index}'}, {'{name}'}, {'{ext}'}
                  </p>
                  <input
                    type="text"
                    className="input"
                    value={fileRenameTemplate}
                    onChange={(e) => saveFileRenameTemplate(e.target.value)}
                    placeholder="{original}"
                  />
                  <p className="text-secondary text-sm">
                    Preview:{' '}
                    {fileRenameTemplate === '{original}'
                      ? 'IMG_1234.jpg (unchanged)'
                      : 'CustomName_001.jpg'}
                  </p>
                </div>
                <button onClick={resetTemplates} className="btn btn-secondary align-self-start">
                  Reset to Defaults
                </button>
              </div>
            </div>
          </section>

          <section>
            <h2>Import Settings</h2>
            <div className="card">
              <div className="flex flex-col gap-md">
                <div className="flex align-center gap-sm">
                  <input
                    type="checkbox"
                    id="auto-eject"
                    checked={autoEject}
                    onChange={toggleAutoEject}
                  />
                  <label htmlFor="auto-eject" className="font-medium">
                    Auto-eject SD cards after successful import
                  </label>
                </div>
              </div>
            </div>
          </section>
        </div>
      </div>
    </>
  )
}
