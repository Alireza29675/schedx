import { useState, useEffect } from 'react'
import { timeAgo } from '../../lib/format'

interface TimeAgoProps {
  date: string
  className?: string
}

export function TimeAgo({ date, className }: TimeAgoProps) {
  const [text, setText] = useState(() => timeAgo(date))

  useEffect(() => {
    const interval = setInterval(() => setText(timeAgo(date)), 15000)
    return () => clearInterval(interval)
  }, [date])

  return <span className={className} title={new Date(date).toLocaleString()}>{text}</span>
}
