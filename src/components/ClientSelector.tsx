import { useCallback, useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { Client } from '../types'
import { CreateClient } from './CreateClient'

interface ClientSelectorProps {
  value: string
  clientId: string | null
  onChange: (clientName: string, clientId: string | null) => void
  required?: boolean
}

export function ClientSelector({ value, clientId, onChange, required }: ClientSelectorProps) {
  const [clients, setClients] = useState<Client[]>([])
  const [query, setQuery] = useState(value)
  const [isOpen, setIsOpen] = useState(false)
  const [showCreateClient, setShowCreateClient] = useState(false)
  const containerRef = useRef<HTMLDivElement>(null)

  const loadClients = useCallback(async () => {
    try {
      const result = await invoke<Client[]>('list_clients', { includeArchived: false })
      setClients(result)
    } catch {
      setClients([])
    }
  }, [])

  useEffect(() => {
    void loadClients()
  }, [loadClients])

  // Keep local query in sync when parent value changes externally
  useEffect(() => {
    setQuery(value)
  }, [value])

  // Close dropdown on outside click
  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setIsOpen(false)
      }
    }
    document.addEventListener('mousedown', handleClick)
    return () => document.removeEventListener('mousedown', handleClick)
  }, [])

  const filtered = clients.filter((c) => c.name.toLowerCase().includes(query.toLowerCase()))

  const handleSelect = (client: Client) => {
    setQuery(client.name)
    setIsOpen(false)
    onChange(client.name, client.id)
  }

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value
    setQuery(val)
    setIsOpen(true)
    const exactMatch = clients.find((c) => c.name.toLowerCase() === val.trim().toLowerCase())
    if (exactMatch) {
      onChange(exactMatch.name, exactMatch.id)
    } else {
      onChange(val, null)
    }
  }

  const handleClientCreated = (client: Client) => {
    setClients((prev) => [...prev, client].sort((a, b) => a.name.localeCompare(b.name)))
    setQuery(client.name)
    setIsOpen(false)
    setShowCreateClient(false)
    onChange(client.name, client.id)
  }

  const showCreateOption =
    query.trim().length > 0 &&
    !clients.some((c) => c.name.toLowerCase() === query.trim().toLowerCase())

  if (showCreateClient) {
    return (
      <div className="client-selector-create">
        <div className="flex flex-col gap-xxs" style={{ marginBottom: 'var(--space-sm)' }}>
          <span className="form-label">Creating new client: &ldquo;{query}&rdquo;</span>
        </div>
        <CreateClient
          onClientCreated={handleClientCreated}
          onCancel={() => setShowCreateClient(false)}
        />
      </div>
    )
  }

  return (
    <div ref={containerRef} className="client-selector">
      <input
        id="clientSelector"
        type="text"
        className="input"
        value={query}
        onChange={handleInputChange}
        onFocus={() => setIsOpen(true)}
        placeholder="Search or type client name"
        required={required}
        autoComplete="off"
      />
      {isOpen && (filtered.length > 0 || showCreateOption) && (
        <div className="client-selector-dropdown">
          {filtered.map((client) => (
            <div
              key={client.id}
              className="client-selector-option"
              onMouseDown={() => handleSelect(client)}
              role="option"
              tabIndex={0}
              aria-selected={client.id === clientId}
            >
              <span className="client-selector-name">{client.name}</span>
              {client.email && (
                <span className="client-selector-email text-secondary">{client.email}</span>
              )}
            </div>
          ))}
          {showCreateOption && (
            <div
              className="client-selector-option client-selector-create-option"
              onMouseDown={() => {
                setIsOpen(false)
                setShowCreateClient(true)
              }}
              role="option"
              tabIndex={0}
              aria-selected={false}
            >
              + Create &ldquo;{query}&rdquo;
            </div>
          )}
        </div>
      )}
    </div>
  )
}
