import { useState, useEffect, useRef, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import {
  sendNotification,
  isPermissionGranted,
  requestPermission,
} from '@tauri-apps/plugin-notification'
import { useNotification } from './useNotification'
import type { SDCard } from '../types'

const AUTO_SCAN_INTERVAL_MS = 5000 // Scan every 5 seconds

interface UseSDCardScannerOptions {
  onCardDetected?: () => void
}

export function useSDCardScanner(options?: UseSDCardScannerOptions) {
  const [sdCards, setSdCards] = useState<SDCard[]>([])
  const [isScanning, setIsScanning] = useState(false)
  const previousCardPaths = useRef<Set<string>>(new Set())
  const permissionGranted = useRef<boolean | null>(null)
  const isInitialScan = useRef(true)
  const { info } = useNotification()
  const onCardDetected = options?.onCardDetected

  const scanForSDCards = useCallback(async () => {
    setIsScanning(true)
    try {
      const cards = await invoke<SDCard[]>('scan_sd_cards')

      // Detect newly mounted cards
      const currentPaths = new Set(cards.map((c) => c.path))
      const newCards = cards.filter((card) => !previousCardPaths.current.has(card.path))

      // Send notification for newly detected cards (skip initial scan to avoid spam on startup)
      if (!isInitialScan.current && newCards.length > 0) {
        for (const card of newCards) {
          // In-app toast notification
          info(`SD Card detected: ${card.name}`)

          // Navigate to import view
          if (onCardDetected) {
            onCardDetected()
          }

          // System notification (if permission granted)
          if (permissionGranted.current === null) {
            permissionGranted.current = await isPermissionGranted()
            if (!permissionGranted.current) {
              const permission = await requestPermission()
              permissionGranted.current = permission === 'granted'
            }
          }

          if (permissionGranted.current) {
            try {
              await sendNotification({
                title: 'SD Card Detected',
                body: `${card.name} has been mounted`,
              })
            } catch (error) {
              console.error('Failed to send system notification:', error)
            }
          }
        }
      }

      previousCardPaths.current = currentPaths
      setSdCards(cards)
      isInitialScan.current = false
    } catch (error) {
      console.error('Failed to scan SD cards:', error)
    } finally {
      setIsScanning(false)
    }
  }, [info, onCardDetected])

  useEffect(() => {
    // Initial scan
    scanForSDCards()

    // Auto-scan for new SD cards every 5 seconds
    const intervalId = setInterval(() => {
      scanForSDCards()
    }, AUTO_SCAN_INTERVAL_MS)

    return () => clearInterval(intervalId)
  }, [scanForSDCards])

  return { sdCards, isScanning, scanForSDCards }
}
