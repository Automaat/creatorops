import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { SDCard } from '../types'

export function Import() {
  const [sdCards, setSdCards] = useState<SDCard[]>([])
  const [isScanning, setIsScanning] = useState(false)

  useEffect(() => {
    scanForSDCards()
  }, [])

  const scanForSDCards = async () => {
    setIsScanning(true)
    try {
      const cards = await invoke<SDCard[]>('scan_sd_cards')
      setSdCards(cards)
    } catch (error) {
      console.error('Failed to scan SD cards:', error)
    } finally {
      setIsScanning(false)
    }
  }

  return (
    <>
      <div className="content-header">
        <div className="flex" style={{ justifyContent: 'space-between', alignItems: 'center' }}>
          <div>
            <h1>Import from SD Card</h1>
            <p className="text-secondary">Detect and import files from SD cards</p>
          </div>
          <button className="btn btn-primary" onClick={scanForSDCards} disabled={isScanning}>
            {isScanning ? 'Scanning...' : 'Refresh'}
          </button>
        </div>
      </div>
      <div className="content-body">
        <div className="flex flex-col gap-lg">
          {sdCards.length === 0 ? (
            <div className="card">
              <p className="text-secondary">
                {isScanning
                  ? 'Scanning for SD cards...'
                  : 'No SD cards detected. Insert an SD card and click Refresh.'}
              </p>
            </div>
          ) : (
            <div className="flex flex-col gap-md">
              {sdCards.map((card) => (
                <SDCardItem key={card.path} card={card} />
              ))}
            </div>
          )}
        </div>
      </div>
    </>
  )
}

interface SDCardItemProps {
  card: SDCard
}

function SDCardItem({ card }: SDCardItemProps) {
  const usedSpace = card.size - card.freeSpace
  const usedPercent = (usedSpace / card.size) * 100

  return (
    <div className="card">
      <div className="flex flex-col gap-md">
        <div className="flex" style={{ justifyContent: 'space-between', alignItems: 'center' }}>
          <div>
            <h3>{card.name}</h3>
            <p className="text-secondary text-sm">{card.path}</p>
          </div>
          <button className="btn btn-primary">Import</button>
        </div>

        <div>
          <div
            className="flex"
            style={{ justifyContent: 'space-between', marginBottom: 'var(--space-xs)' }}
          >
            <span className="text-sm text-secondary">{card.fileCount} files</span>
            <span className="text-sm text-secondary">
              {formatBytes(usedSpace)} / {formatBytes(card.size)}
            </span>
          </div>
          <div className="progress">
            <div className="progress-bar" style={{ width: `${usedPercent}%` }} />
          </div>
        </div>
      </div>
    </div>
  )
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`
}
