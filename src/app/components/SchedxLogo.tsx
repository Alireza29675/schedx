import React from 'react'

type SchedxIconVariant = 'dark' | 'light'

interface SchedxIconProps {
  size?: number
  variant?: SchedxIconVariant
  animated?: boolean
}

const COLORS: Record<SchedxIconVariant, { x: string; bg: string }> = {
  dark: { x: '#808080', bg: '#505050' },
  light: { x: '#505050', bg: '#D2D2D2' },
}

// X pattern: diagonals are "x" color, off-diagonals are "bg" color
const IS_X = [
  [true, false, true],
  [false, true, false],
  [true, false, true],
]

// Per-cell animation IDs — null means no animation (bg cells)
const CELL_ANIM: (string | null)[][] = [
  ['sx-00', null, 'sx-02'],
  [null, 'sx-11', null],
  ['sx-20', null, 'sx-22'],
]

export const SchedxIcon = ({ size = 46, variant = 'dark', animated = false }: SchedxIconProps) => {
  const { x, bg } = COLORS[variant]
  const gap = size / 10
  const cell = (size - gap * 2) / 3
  const step = cell + gap

  // 5s cycle: rest 0.3s → comet 0.5s → hold 3s → fade 1s → rest 0.2s → loop
  //
  // Comet 1 (main diag):
  //   (0,0) lights at 6%        (~0.30s)
  //   (1,1)+(2,2) light at 9%   (~0.45s)  together
  //
  // Comet 2 (anti diag):
  //   (0,2) lights at 12%       (~0.60s)
  //   (1,1) already white
  //   (2,0) lights at 14%       (~0.70s)
  //
  // Each cell glows for 0.15s (3%) on impact then glow fades
  // Hold white until 76% (3.8s), fade to rest by 96% (4.8s)
  const glow = 'drop-shadow(0 0 6px #FFFFFF)'
  const noFx = 'none'
  const keyframes = animated ? `
    @keyframes sx-00 {
      0%, 5% { fill: ${x}; filter: ${noFx} }
      6% { fill: #FFF; filter: ${glow} }
      9%, 76% { fill: #FFF; filter: ${noFx} }
      96%, 100% { fill: ${x}; filter: ${noFx} }
    }
    @keyframes sx-11 {
      0%, 8% { fill: ${x}; filter: ${noFx} }
      9% { fill: #FFF; filter: ${glow} }
      12%, 76% { fill: #FFF; filter: ${noFx} }
      96%, 100% { fill: ${x}; filter: ${noFx} }
    }
    @keyframes sx-22 {
      0%, 8% { fill: ${x}; filter: ${noFx} }
      9% { fill: #FFF; filter: ${glow} }
      12%, 76% { fill: #FFF; filter: ${noFx} }
      96%, 100% { fill: ${x}; filter: ${noFx} }
    }
    @keyframes sx-02 {
      0%, 11% { fill: ${x}; filter: ${noFx} }
      12% { fill: #FFF; filter: ${glow} }
      15%, 76% { fill: #FFF; filter: ${noFx} }
      96%, 100% { fill: ${x}; filter: ${noFx} }
    }
    @keyframes sx-20 {
      0%, 13% { fill: ${x}; filter: ${noFx} }
      14% { fill: #FFF; filter: ${glow} }
      17%, 76% { fill: #FFF; filter: ${noFx} }
      96%, 100% { fill: ${x}; filter: ${noFx} }
    }
  ` : ''

  return (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} xmlns="http://www.w3.org/2000/svg" style={{ overflow: 'visible' }}>
      {animated && <style>{keyframes}</style>}
      {IS_X.flatMap((row, r) =>
        row.map((isX, c) => {
          const anim = CELL_ANIM[r][c]
          return (
            <rect
              key={`${r}-${c}`}
              x={c * step}
              y={r * step}
              width={cell}
              height={cell}
              fill={isX ? x : bg}
              style={animated && anim ? { animation: `${anim} 5s ease-in-out infinite` } : undefined}
            />
          )
        })
      )}
    </svg>
  )
}

export const SchedxLogo = () => {
  return (
    <h1 className='logo' aria-label='schedx'>sched<SchedxIcon size={46} animated /></h1>
  )
}