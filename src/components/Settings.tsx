import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { open as openUrl } from '@tauri-apps/plugin-shell'
import { useTheme } from '../hooks/useTheme'
import { useNotification } from '../hooks/useNotification'
import type { BackupDestination, DeliveryDestination, GoogleDriveAccount } from '../types'

const DEFAULT_FOLDER_TEMPLATE = '{YYYY}-{MM}-{DD}_{ClientName}_{Type}'
const DEFAULT_FILE_TEMPLATE = '{original}'

export function Settings() {
  const { theme, setTheme } = useTheme()
  const { error: showError, success: showSuccess } = useNotification()
  const [destinations, setDestinations] = useState<BackupDestination[]>([])
  const [newDestName, setNewDestName] = useState('')
  const [deliveryDestinations, setDeliveryDestinations] = useState<DeliveryDestination[]>([])
  const [newDeliveryDestName, setNewDeliveryDestName] = useState('')
  const [defaultImportLocation, setDefaultImportLocation] = useState('')
  const [archiveLocation, setArchiveLocation] = useState('')
  const [folderTemplate, setFolderTemplate] = useState(DEFAULT_FOLDER_TEMPLATE)
  const [fileRenameTemplate, setFileRenameTemplate] = useState(DEFAULT_FILE_TEMPLATE)
  const [autoEject, setAutoEject] = useState(false)
  const [driveAccount, setDriveAccount] = useState<GoogleDriveAccount | null>(null)
  const [driveConflictMode, setDriveConflictMode] = useState<'overwrite' | 'rename' | 'skip'>(
    'rename'
  )
  const [connectingDrive, setConnectingDrive] = useState(false)

  useEffect(() => {
    loadDestinations()
    loadDeliveryDestinations()
    loadDefaultImportLocation()
    loadArchiveLocation()
    loadTemplates()
    loadAutoEject()
    loadDriveAccount().catch(console.error)
    loadDriveConflictMode()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  function loadDestinations() {
    try {
      const stored = localStorage.getItem('backup_destinations')
      if (stored) {
        const parsed: unknown = JSON.parse(stored)
        if (
          Array.isArray(parsed) &&
          parsed.every(
            (item): item is BackupDestination =>
              typeof item === 'object' &&
              item !== null &&
              'id' in item &&
              'name' in item &&
              'path' in item
          )
        ) {
          setDestinations(parsed)
        }
      }
    } catch (error) {
      console.error('Failed to load destinations:', error)
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
          createdAt: new Date().toISOString(),
          enabled: true,
          id: crypto.randomUUID(),
          name: newDestName.trim(),
          path: selected,
        }

        saveDestinations([...destinations, newDest])
        setNewDestName('')
      }
    } catch (error) {
      console.error('Failed to add destination:', error)
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
        const parsed: unknown = JSON.parse(stored)
        if (Array.isArray(parsed)) {
          const migrated = parsed.map((item): DeliveryDestination => {
            if (typeof item === 'object' && item !== null) {
              if ('type' in item) {
                return item as DeliveryDestination
              }
              if ('path' in item && 'id' in item && 'name' in item) {
                return {
                  type: 'local',
                  id: item.id as string,
                  name: item.name as string,
                  path: item.path as string,
                  enabled: (item.enabled as boolean) ?? true,
                  createdAt: (item.createdAt as string) ?? new Date().toISOString(),
                }
              }
            }
            throw new Error('Invalid destination format')
          })
          localStorage.setItem('delivery_destinations', JSON.stringify(migrated))
          setDeliveryDestinations(migrated)
        }
      }
    } catch (error) {
      console.error('Failed to load delivery destinations:', error)
    }
  }

  function loadDefaultImportLocation() {
    try {
      const stored = localStorage.getItem('default_import_location')
      if (stored) {
        setDefaultImportLocation(stored)
      }
    } catch (error) {
      console.error('Failed to load default import location:', error)
    }
  }

  function loadArchiveLocation() {
    try {
      const stored = localStorage.getItem('archive_location')
      if (stored) {
        setArchiveLocation(stored)
      }
    } catch (error) {
      console.error('Failed to load archive location:', error)
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
          type: 'local',
          createdAt: new Date().toISOString(),
          enabled: true,
          id: crypto.randomUUID(),
          name: newDeliveryDestName.trim(),
          path: selected,
        }

        saveDeliveryDestinations([...deliveryDestinations, newDest])
        setNewDeliveryDestName('')
      }
    } catch (error) {
      console.error('Failed to add delivery destination:', error)
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

  async function selectStorageLocation(storageKey: string, setter: (value: string) => void) {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
      })

      if (selected) {
        localStorage.setItem(storageKey, selected)
        setter(selected)
      }
    } catch (error) {
      console.error(`Failed to select ${storageKey}:`, error)
    }
  }

  async function selectDefaultImportLocation() {
    await selectStorageLocation('default_import_location', setDefaultImportLocation)
  }

  async function selectArchiveLocation() {
    await selectStorageLocation('archive_location', setArchiveLocation)
  }

  function loadTemplates() {
    try {
      const storedFolderTemplate = localStorage.getItem('folder_template')
      const storedFileTemplate = localStorage.getItem('file_rename_template')
      if (storedFolderTemplate) {
        setFolderTemplate(storedFolderTemplate)
      }
      if (storedFileTemplate) {
        setFileRenameTemplate(storedFileTemplate)
      }
    } catch (error) {
      console.error('Failed to load templates:', error)
      showError('Failed to load template settings')
    }
  }

  function loadAutoEject() {
    try {
      const stored = localStorage.getItem('auto_eject')
      if (stored) {
        setAutoEject(stored === 'true')
      }
    } catch (error) {
      console.error('Failed to load auto-eject setting:', error)
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

  async function loadDriveAccount() {
    try {
      const account = await invoke<GoogleDriveAccount | null>('get_google_drive_account')
      setDriveAccount(account)
    } catch (error) {
      console.error('Failed to load Google Drive account:', error)
    }
  }

  function loadDriveConflictMode() {
    try {
      const stored = localStorage.getItem('drive_conflict_mode')
      if (stored && (stored === 'overwrite' || stored === 'rename' || stored === 'skip')) {
        setDriveConflictMode(stored)
      }
    } catch (error) {
      console.error('Failed to load conflict mode:', error)
    }
  }

  function saveDriveConflictMode(mode: 'overwrite' | 'rename' | 'skip') {
    localStorage.setItem('drive_conflict_mode', mode)
    setDriveConflictMode(mode)
  }

  async function handleConnectDrive() {
    try {
      setConnectingDrive(true)
      const { authUrl } = await invoke<{ authUrl: string }>('start_google_drive_auth')
      await openUrl(authUrl)
      const account = await invoke<GoogleDriveAccount>('complete_google_drive_auth')
      setDriveAccount(account)
      showSuccess('Google Drive connected successfully')
    } catch (error) {
      console.error('Failed to connect Google Drive:', error)
      showError('Failed to connect Google Drive')
    } finally {
      setConnectingDrive(false)
    }
  }

  async function handleDisconnectDrive() {
    try {
      await invoke('remove_google_drive_account')
      setDriveAccount(null)
      showSuccess('Google Drive disconnected')
    } catch (error) {
      console.error('Failed to disconnect Google Drive:', error)
      showError('Failed to disconnect Google Drive')
    }
  }

  async function handleConfigureParentFolder() {
    try {
      const folderId = prompt('Enter Google Drive parent folder ID (or leave empty for root):')
      await invoke('set_drive_parent_folder', {
        folderId: folderId?.trim() || null,
      })
      await loadDriveAccount()
      showSuccess('Parent folder configured')
    } catch (error) {
      console.error('Failed to configure parent folder:', error)
      showError('Failed to configure parent folder')
    }
  }

  async function handleTestConnection() {
    try {
      await invoke('test_google_drive_connection')
      showSuccess('Connection test successful')
    } catch (error) {
      console.error('Connection test failed:', error)
      showError('Connection test failed')
    }
  }

  return (
    <div className="content-body">
      <div className="settings-header">
        <h1>Settings</h1>
        <p className="text-secondary">Configure CreatorOps preferences</p>
      </div>
      <div className="settings-content">
        <div className="flex flex-col">
          <section>
            <h2>Appearance</h2>
            <div className="card">
              <div className="flex flex-col gap-md">
                <div>
                  <div className="font-medium">Theme</div>
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
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') {
                        void addDestination()
                      }
                    }}
                  />
                  <button onClick={() => void addDestination()} className="btn btn-primary">
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
                            <span className="font-medium">
                              {dest.type === 'google-drive' && '☁️ '}
                              {dest.name}
                            </span>
                          </div>
                          <p className="text-secondary text-sm">
                            {dest.type === 'local' ? dest.path : 'Google Drive'}
                          </p>
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
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') {
                        void addDeliveryDestination()
                      }
                    }}
                  />
                  <button onClick={() => void addDeliveryDestination()} className="btn btn-primary">
                    Add Destination
                  </button>
                </div>
              </div>
            </div>
          </section>

          <section>
            <h2>Google Drive Integration</h2>
            <div className="card">
              <div className="flex flex-col gap-md">
                <p className="text-secondary text-sm">
                  Connect Google Drive for cloud-based client deliveries
                </p>

                {driveAccount ? (
                  <>
                    <div className="drive-account-info">
                      <div className="drive-account-details">
                        <div className="font-medium">{driveAccount.displayName}</div>
                        <p className="text-secondary text-sm">{driveAccount.email}</p>
                        {driveAccount.parentFolderId && (
                          <p className="text-secondary text-sm">
                            Parent folder: {driveAccount.parentFolderId}
                          </p>
                        )}
                      </div>
                      <div className="drive-actions">
                        <button
                          onClick={() => void handleConfigureParentFolder()}
                          className="btn btn-secondary btn-sm"
                        >
                          Configure Parent Folder
                        </button>
                        <button
                          onClick={() => void handleTestConnection()}
                          className="btn btn-secondary btn-sm"
                        >
                          Test Connection
                        </button>
                        <button
                          onClick={() => void handleDisconnectDrive()}
                          className="btn btn-secondary btn-sm"
                        >
                          Disconnect
                        </button>
                      </div>
                    </div>

                    <div className="drive-conflict-mode">
                      <h4 className="card-section-label">File Conflict Handling</h4>
                      <p
                        className="text-secondary text-sm"
                        style={{ marginBottom: 'var(--space-sm)' }}
                      >
                        What to do when a file already exists in Google Drive
                      </p>
                      <div className="flex flex-col gap-sm">
                        <label className="flex align-center gap-sm">
                          <input
                            type="radio"
                            name="conflict-mode"
                            value="overwrite"
                            checked={driveConflictMode === 'overwrite'}
                            onChange={(e) =>
                              saveDriveConflictMode(
                                e.target.value as 'overwrite' | 'rename' | 'skip'
                              )
                            }
                          />
                          <span>Overwrite existing files</span>
                        </label>
                        <label className="flex align-center gap-sm">
                          <input
                            type="radio"
                            name="conflict-mode"
                            value="rename"
                            checked={driveConflictMode === 'rename'}
                            onChange={(e) =>
                              saveDriveConflictMode(
                                e.target.value as 'overwrite' | 'rename' | 'skip'
                              )
                            }
                          />
                          <span>Rename new files (e.g., photo (1).jpg)</span>
                        </label>
                        <label className="flex align-center gap-sm">
                          <input
                            type="radio"
                            name="conflict-mode"
                            value="skip"
                            checked={driveConflictMode === 'skip'}
                            onChange={(e) =>
                              saveDriveConflictMode(
                                e.target.value as 'overwrite' | 'rename' | 'skip'
                              )
                            }
                          />
                          <span>Skip existing files</span>
                        </label>
                      </div>
                    </div>
                  </>
                ) : (
                  <button
                    onClick={() => void handleConnectDrive()}
                    className="btn btn-primary"
                    disabled={connectingDrive}
                  >
                    {connectingDrive ? 'Connecting...' : 'Connect Google Drive'}
                  </button>
                )}
              </div>
            </div>
          </section>

          <section>
            <h2>Storage Paths</h2>
            <div className="card">
              <div className="flex flex-col">
                <div className="card-section">
                  <h4 className="card-section-label">Default Import Location</h4>
                  <div className="flex gap-md align-center">
                    <p className="text-secondary text-sm" style={{ margin: 0 }}>
                      {defaultImportLocation || '~/CreatorOps/Projects'}
                    </p>
                    <button
                      onClick={() => void selectDefaultImportLocation()}
                      className="btn btn-primary"
                      style={{ marginLeft: 'auto' }}
                    >
                      {defaultImportLocation ? 'Change Location' : 'Select Location'}
                    </button>
                  </div>
                </div>

                <div className="card-section">
                  <h4 className="card-section-label">Archive Location</h4>
                  <div className="flex gap-md align-center">
                    <p className="text-secondary text-sm" style={{ margin: 0 }}>
                      {archiveLocation || 'Not configured'}
                    </p>
                    <button
                      onClick={() => void selectArchiveLocation()}
                      className="btn btn-primary"
                      style={{ marginLeft: 'auto' }}
                    >
                      Select Location
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </section>

          <section>
            <h2>Folder Templates</h2>
            <div className="card">
              <div className="flex flex-col">
                <div className="card-section">
                  <h4 className="card-section-label">Project Folder</h4>
                  <div className="flex flex-col gap-xs">
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
                </div>

                <div className="card-section">
                  <h4 className="card-section-label">File Rename</h4>
                  <div className="flex flex-col gap-xs">
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
                </div>

                <div className="card-section">
                  <button onClick={resetTemplates} className="btn btn-secondary align-self-end">
                    Reset to Defaults
                  </button>
                </div>
              </div>
            </div>
          </section>

          <section>
            <h2>Import</h2>
            <div className="card">
              <div className="flex flex-col gap-md">
                <div className="flex align-center gap-sm">
                  <input
                    type="checkbox"
                    id="auto-eject"
                    checked={autoEject}
                    onChange={toggleAutoEject}
                  />
                  <label htmlFor="auto-eject">Auto-eject SD cards after successful import</label>
                </div>
              </div>
            </div>
          </section>
        </div>
      </div>
    </div>
  )
}
