export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) {return '0 B'}
  if (bytes === 0) {return '0 B'}
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`
}

export function formatSpeed(bytesPerSecond: number): string {
  return `${formatBytes(bytesPerSecond)}/s`
}

export function formatETA(seconds: number): string {
  if (seconds === 0) {return '--'}
  const hrs = Math.floor(seconds / 3600)
  const mins = Math.floor((seconds % 3600) / 60)
  const secs = seconds % 60
  if (hrs > 0) {return `${hrs}h ${mins}m`}
  if (mins > 0) {return `${mins}m ${secs}s`}
  return `${secs}s`
}

export function formatDate(dateString: string): string {
  try {
    const timestamp = Number.parseInt(dateString, 10) * 1000
    if (isNaN(timestamp)) {return dateString}

    const date = new Date(timestamp)
    return date.toLocaleString('en-US', {
      day: 'numeric', hour: '2-digit', minute: '2-digit', month: 'short', year: 'numeric',
    })
  } catch {
    return dateString
  }
}

export function formatDateShort(dateString: string): string {
  try {
    const date = new Date(dateString)
    if (isNaN(date.getTime())) {return dateString}

    const day = date.getDate()
    const month = date.toLocaleString('en-US', { month: 'short' })
    const year = date.getFullYear()

    return `${day} ${month} ${year}`
  } catch {
    return dateString
  }
}

const MONTH_NAMES_SHORT = [
  'Jan',
  'Feb',
  'Mar',
  'Apr',
  'May',
  'Jun',
  'Jul',
  'Aug',
  'Sep',
  'Oct',
  'Nov',
  'Dec',
] as const

const MONTH_NAMES_FULL = [
  'January',
  'February',
  'March',
  'April',
  'May',
  'June',
  'July',
  'August',
  'September',
  'October',
  'November',
  'December',
] as const

export function formatDisplayDate(date: Date | string): string {
  const d = typeof date === 'string' ? new Date(date) : date
  return `${MONTH_NAMES_SHORT[d.getMonth()]} ${d.getDate()}, ${d.getFullYear()}`
}

export { MONTH_NAMES_SHORT, MONTH_NAMES_FULL }
