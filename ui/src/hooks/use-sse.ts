import { useEffect, useRef } from 'react'

type SSEHandler = (event: string, data: string) => void

export function useSSE(handler: SSEHandler) {
  const handlerRef = useRef(handler)
  handlerRef.current = handler

  useEffect(() => {
    const token = document.querySelector('meta[name="schedx-token"]')?.getAttribute('content') ?? ''
    const source = new EventSource(`/api/events?token=${encodeURIComponent(token)}`)

    source.addEventListener('job-changed', (e) => handlerRef.current('job-changed', e.data))
    source.addEventListener('run-started', (e) => handlerRef.current('run-started', e.data))
    source.addEventListener('run-completed', (e) => handlerRef.current('run-completed', e.data))
    source.addEventListener('config-changed', (e) => handlerRef.current('config-changed', e.data))

    return () => source.close()
  }, [])
}
