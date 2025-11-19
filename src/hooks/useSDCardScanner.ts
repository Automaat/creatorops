import { useState, useEffect, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { sendNotification } from '@tauri-apps/plugin-notification'
import type { SDCard } from '../types'

const AUTO_SCAN_INTERVAL_MS = 5000 // Scan every 5 seconds

export function useSDCardScanner() {
  const [sdCards, setSdCards] = useState<SDCard[]>([])
  const [isScanning, setIsScanning] = useState(false)
  const previousCardPaths = useRef<Set<string>>(new Set())

  const scanForSDCards = async () => {
    setIsScanning(true)
    try {
      const cards = await invoke<SDCard[]>('scan_sd_cards')

      // Detect newly mounted cards
      const currentPaths = new Set(cards.map((c) => c.path))
      const newCards = cards.filter((card) => !previousCardPaths.current.has(card.path))

      // Send notification for newly detected cards
      if (previousCardPaths.current.size > 0 && newCards.length > 0) {
        for (const card of newCards) {
          try {
            await sendNotification({
              title: 'SD Card Detected',
              body: `${card.name} (${card.deviceType}) has been mounted`,
            })
          } catch (error) {
            console.error('Failed to send notification:', error)
          }
        }
      }

      previousCardPaths.current = currentPaths
      setSdCards(cards)
    } catch (error) {
      console.error('Failed to scan SD cards:', error)
    } finally {
      setIsScanning(false)
    }
  }

  useEffect(() => {
    // Initial scan
    scanForSDCards()

    // Auto-scan for new SD cards every 5 seconds
    const intervalId = setInterval(() => {
      scanForSDCards()
    }, AUTO_SCAN_INTERVAL_MS)

    return () => clearInterval(intervalId)
  }, [])

  return { sdCards, isScanning, scanForSDCards }
}
