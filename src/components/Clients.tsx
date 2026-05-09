import { useCallback, useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { Client, ClientWithProjects } from '../types'
import { useNotification } from '../hooks/useNotification'
import { CreateClient } from './CreateClient'

type FilterMode = 'active' | 'archived' | 'all'

interface ClientsProps {
  isActive?: boolean
}

export function Clients({ isActive }: ClientsProps) {
  const { error: showError, success } = useNotification()
  const [clients, setClients] = useState<Client[]>([])
  const [loading, setLoading] = useState(true)
  const [selectedClient, setSelectedClient] = useState<ClientWithProjects | null>(null)
  const [searchQuery, setSearchQuery] = useState('')
  const [filterMode, setFilterMode] = useState<FilterMode>('active')
  const [showCreateClient, setShowCreateClient] = useState(false)
  const [showDeleteDialog, setShowDeleteDialog] = useState(false)
  const [isDeleting, setIsDeleting] = useState(false)

  const loadClients = useCallback(async () => {
    try {
      setLoading(true)
      const includeArchived = filterMode !== 'active'
      const result = await invoke<Client[]>('list_clients', { includeArchived })
      const filtered = filterMode === 'all' ? result : result.filter((c) => c.status === filterMode)
      setClients(filtered)
    } catch (err) {
      console.error('Failed to load clients:', err)
      if (isActive) showError('Failed to load clients')
    } finally {
      setLoading(false)
    }
  }, [filterMode, isActive, showError])

  useEffect(() => {
    void loadClients()
  }, [loadClients])

  const handleSelectClient = async (client: Client) => {
    try {
      const detail = await invoke<ClientWithProjects>('get_client', { clientId: client.id })
      setSelectedClient(detail)
    } catch (err) {
      console.error('Failed to load client detail:', err)
      showError('Failed to load client details')
    }
  }

  const handleBack = () => {
    setSelectedClient(null)
    setShowDeleteDialog(false)
  }

  const handleClientCreated = (client: Client) => {
    setShowCreateClient(false)
    void loadClients()
    void handleSelectClient(client)
  }

  const handleArchiveToggle = async () => {
    if (!selectedClient) return
    const newStatus = selectedClient.status === 'active' ? 'archived' : 'active'
    try {
      await invoke<Client>('update_client_status', {
        clientId: selectedClient.id,
        status: newStatus,
      })
      success(newStatus === 'archived' ? 'Client archived' : 'Client restored')
      void loadClients()
      setSelectedClient(null)
    } catch (err) {
      console.error('Failed to update client status:', err)
      showError('Failed to update client status')
    }
  }

  const handleDelete = async () => {
    if (!selectedClient) return
    setIsDeleting(true)
    try {
      await invoke<void>('delete_client', { clientId: selectedClient.id })
      success('Client deleted')
      setShowDeleteDialog(false)
      setSelectedClient(null)
      void loadClients()
    } catch (err) {
      console.error('Failed to delete client:', err)
      showError(err instanceof Error ? err.message : String(err))
    } finally {
      setIsDeleting(false)
    }
  }

  const displayedClients = clients.filter(
    (c) =>
      c.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      c.email?.toLowerCase().includes(searchQuery.toLowerCase())
  )

  if (loading) {
    return <div className="loading">Loading...</div>
  }

  if (selectedClient) {
    return (
      <ClientDetail
        client={selectedClient}
        onBack={handleBack}
        onArchiveToggle={() => void handleArchiveToggle()}
        onDeleteRequest={() => setShowDeleteDialog(true)}
        showDeleteDialog={showDeleteDialog}
        isDeleting={isDeleting}
        onDeleteConfirm={() => void handleDelete()}
        onDeleteCancel={() => setShowDeleteDialog(false)}
      />
    )
  }

  return (
    <>
      <div className="content-header">
        <h1>Clients</h1>
        <p className="text-secondary">Manage your photography clients</p>
        <button className="btn-primary" onClick={() => setShowCreateClient(true)}>
          Create Client
        </button>
      </div>

      <div className="content-body">
        <div className="flex flex-col gap-lg">
          <div className="flex gap-md" style={{ alignItems: 'center' }}>
            <input
              type="text"
              className="input"
              placeholder="Search clients..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              style={{ flex: 1 }}
            />
            <div className="flex gap-xs">
              {(['active', 'archived', 'all'] as FilterMode[]).map((mode) => (
                <button
                  key={mode}
                  className={`btn ${filterMode === mode ? 'btn-primary' : 'btn-ghost'}`}
                  onClick={() => setFilterMode(mode)}
                >
                  {mode.charAt(0).toUpperCase() + mode.slice(1)}
                </button>
              ))}
            </div>
          </div>

          {displayedClients.length === 0 ? (
            <div className="empty-state">
              <p className="text-secondary">
                {searchQuery ? 'No clients match your search' : 'No clients yet'}
              </p>
              {!searchQuery && (
                <p className="text-secondary text-sm">Click Create Client to get started</p>
              )}
            </div>
          ) : (
            <div className="client-grid">
              {displayedClients.map((client) => (
                <ClientCard
                  key={client.id}
                  client={client}
                  onClick={() => void handleSelectClient(client)}
                />
              ))}
            </div>
          )}
        </div>
      </div>

      {showCreateClient && (
        <div
          className="dialog-overlay"
          onClick={() => setShowCreateClient(false)}
          role="presentation"
        >
          <div
            className="dialog"
            onClick={(e) => e.stopPropagation()}
            onKeyDown={(e) => e.key === 'Escape' && setShowCreateClient(false)}
            role="dialog"
          >
            <h2>Create New Client</h2>
            <CreateClient
              onClientCreated={handleClientCreated}
              onCancel={() => setShowCreateClient(false)}
            />
          </div>
        </div>
      )}
    </>
  )
}

interface ClientCardProps {
  client: Client
  onClick: () => void
}

function ClientCard({ client, onClick }: ClientCardProps) {
  return (
    <div
      className="client-card"
      onClick={onClick}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => e.key === 'Enter' && onClick()}
    >
      <div className="client-card-header">
        <h3 className="client-card-name">{client.name}</h3>
        {client.status === 'archived' && <span className="badge badge-secondary">Archived</span>}
      </div>
      {client.email && <p className="text-secondary text-sm">{client.email}</p>}
      {client.phone && <p className="text-secondary text-sm">{client.phone}</p>}
    </div>
  )
}

interface ClientDetailProps {
  client: ClientWithProjects
  onBack: () => void
  onArchiveToggle: () => void
  onDeleteRequest: () => void
  showDeleteDialog: boolean
  isDeleting: boolean
  onDeleteConfirm: () => void
  onDeleteCancel: () => void
}

function ClientDetail({
  client,
  onBack,
  onArchiveToggle,
  onDeleteRequest,
  showDeleteDialog,
  isDeleting,
  onDeleteConfirm,
  onDeleteCancel,
}: ClientDetailProps) {
  return (
    <>
      <div className="content-header">
        <div className="flex gap-md" style={{ alignItems: 'center' }}>
          <button className="btn btn-ghost" onClick={onBack}>
            ← Back
          </button>
          <h1>{client.name}</h1>
        </div>
        <div className="flex gap-md">
          <button className="btn btn-ghost" onClick={onArchiveToggle}>
            {client.status === 'active' ? 'Archive' : 'Restore'}
          </button>
        </div>
      </div>

      <div className="content-body">
        <div className="flex flex-col gap-xl">
          <section className="client-detail-meta">
            {client.email && (
              <p>
                <span className="text-secondary">Email: </span>
                <a href={`mailto:${client.email}`}>{client.email}</a>
              </p>
            )}
            {client.phone && (
              <p>
                <span className="text-secondary">Phone: </span>
                {client.phone}
              </p>
            )}
            {client.notes && (
              <div className="client-detail-notes">
                <p className="text-secondary" style={{ marginBottom: 'var(--space-xs)' }}>
                  Notes
                </p>
                <p>{client.notes}</p>
              </div>
            )}
          </section>

          <section>
            <h2>Projects ({client.projects.length})</h2>
            {client.projects.length === 0 ? (
              <div className="empty-state">
                <p className="text-secondary">No projects for this client yet</p>
              </div>
            ) : (
              <div className="project-list">
                {client.projects.map((project) => (
                  <div key={project.id} className="project-list-item">
                    <div className="project-list-content">
                      <div>
                        <h3>{project.name}</h3>
                        <p className="text-secondary text-sm">{project.date}</p>
                      </div>
                      <span className="project-status">{project.status}</span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </section>

          {client.projects.length === 0 && (
            <section>
              <button type="button" className="btn btn-danger" onClick={onDeleteRequest}>
                Delete Client
              </button>
            </section>
          )}
        </div>
      </div>

      {showDeleteDialog && (
        <div className="dialog-overlay" onClick={onDeleteCancel} role="presentation">
          <div className="dialog" onClick={(e) => e.stopPropagation()} role="dialog">
            <h2>Delete Client</h2>
            <p>
              Are you sure you want to delete <strong>{client.name}</strong>? This cannot be undone.
            </p>
            <div
              className="flex gap-md"
              style={{ justifyContent: 'flex-end', marginTop: 'var(--space-lg)' }}
            >
              <button className="btn btn-ghost" onClick={onDeleteCancel} disabled={isDeleting}>
                Cancel
              </button>
              <button className="btn btn-danger" onClick={onDeleteConfirm} disabled={isDeleting}>
                {isDeleting ? 'Deleting...' : 'Delete'}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  )
}
