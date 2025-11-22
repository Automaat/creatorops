import { useState, useRef, useEffect, type ReactNode } from 'react'
import calendarIcon from '../assets/icons/calendar.png'
import { formatDisplayDate, MONTH_NAMES_FULL } from '../utils/formatting'

interface DatePickerProps {
  value: string
  onChange: (date: string) => void
  label?: string | ReactNode
  required?: boolean
  id?: string
  autoOpen?: boolean
}

export function DatePicker({ value, onChange, label, required, id, autoOpen }: DatePickerProps) {
  const [isOpen, setIsOpen] = useState(autoOpen ?? false)
  const [selectedDate, setSelectedDate] = useState<Date | null>(
    value && value.trim() ? new Date(value) : null
  )
  const [viewDate, setViewDate] = useState(
    value && value.trim() ? new Date(value) : new Date()
  )
  const containerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (value && value.trim()) {
      const date = new Date(value)
      if (!isNaN(date.getTime())) {
        setSelectedDate(date)
        setViewDate(date)
      }
    } else {
      setSelectedDate(null)
    }
  }, [value])

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        if (selectedDate) {
          onChange(formatDate(selectedDate))
        }
        setIsOpen(false)
      }
    }

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside)
      return () => document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [isOpen, selectedDate, onChange])

  const formatDate = (date: Date) => {
    const month = String(date.getMonth() + 1).padStart(2, '0')
    const day = String(date.getDate()).padStart(2, '0')
    return `${date.getFullYear()}-${month}-${day}`
  }

  const handleDateSelect = (day: number) => {
    const newDate = new Date(viewDate.getFullYear(), viewDate.getMonth(), day)
    setSelectedDate(newDate)
    onChange(formatDate(newDate))
    setIsOpen(false)
  }

  const previousMonth = () => {
    setViewDate(new Date(viewDate.getFullYear(), viewDate.getMonth() - 1))
  }

  const nextMonth = () => {
    setViewDate(new Date(viewDate.getFullYear(), viewDate.getMonth() + 1))
  }

  const getDaysInMonth = () => {
    const year = viewDate.getFullYear()
    const month = viewDate.getMonth()
    const firstDay = new Date(year, month, 1).getDay()
    const daysInMonth = new Date(year, month + 1, 0).getDate()
    const days: (number | null)[] = []

    // Add empty cells for days before the first day
    for (let i = 0; i < firstDay; i++) {
      days.push(null)
    }

    // Add all days in month
    for (let i = 1; i <= daysInMonth; i++) {
      days.push(i)
    }

    return days
  }

  const isToday = (day: number) => {
    const today = new Date()
    return (
      day === today.getDate() &&
      viewDate.getMonth() === today.getMonth() &&
      viewDate.getFullYear() === today.getFullYear()
    )
  }

  const isSelected = (day: number) => {
    if (!selectedDate) return false
    return (
      day === selectedDate.getDate() &&
      viewDate.getMonth() === selectedDate.getMonth() &&
      viewDate.getFullYear() === selectedDate.getFullYear()
    )
  }

  const weekDays = ['Su', 'Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa']

  return (
    <div className="date-picker-container" ref={containerRef}>
      {label && (
        <label htmlFor={id} className="form-label">
          {label} {required && '*'}
        </label>
      )}
      <button
        type="button"
        className="date-picker-input"
        onClick={() => setIsOpen(!isOpen)}
        id={id}
      >
        <span className={selectedDate ? 'date-picker-value' : 'date-picker-placeholder'}>
          {selectedDate ? formatDisplayDate(selectedDate) : 'Select date'}
        </span>
        <img src={calendarIcon} alt="Calendar" className="date-picker-icon" />
      </button>

      {isOpen && (
        <div className="date-picker-dropdown">
          <div className="date-picker-header">
            <button type="button" onClick={previousMonth} className="date-picker-nav">
              ←
            </button>
            <span className="date-picker-title">
              {MONTH_NAMES_FULL[viewDate.getMonth()]} {viewDate.getFullYear()}
            </span>
            <button type="button" onClick={nextMonth} className="date-picker-nav">
              →
            </button>
          </div>

          <div className="date-picker-grid">
            {weekDays.map((day) => (
              <div key={day} className="date-picker-weekday">
                {day}
              </div>
            ))}
            {getDaysInMonth().map((day, index) => (
              <button
                key={index}
                type="button"
                className={`date-picker-day ${day === null ? 'empty' : ''} ${
                  day && isSelected(day) ? 'selected' : ''
                } ${day && isToday(day) ? 'today' : ''}`}
                onClick={() => day && handleDateSelect(day)}
                disabled={day === null}
              >
                {day}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}
