import { useEffect, useState } from 'react'

type Theme = 'system' | 'light' | 'dark'

function isValidTheme(value: string | null): value is Theme {
  return value === 'system' || value === 'light' || value === 'dark'
}

export function useTheme() {
  const [theme, setTheme] = useState<Theme>(() => {
    const stored = localStorage.getItem('theme')
    return isValidTheme(stored) ? stored : 'system'
  })

  useEffect(() => {
    const root = document.documentElement

    if (theme === 'light') {
      root.setAttribute('data-theme', 'light')
    } else if (theme === 'dark') {
      root.setAttribute('data-theme', 'dark')
    } else {
      root.removeAttribute('data-theme')
    }

    localStorage.setItem('theme', theme)
  }, [theme])

  return { setTheme, theme }
}
