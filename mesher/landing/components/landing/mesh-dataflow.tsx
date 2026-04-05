"use client"

import { useEffect, useRef, useState } from "react"
import { motion } from "framer-motion"

// ─── Palette ─────────────────────────────────────────────────────────────────
const G: [number, number, number] = [89, 193, 132]   // accent green
const A: [number, number, number] = [245, 158, 11]    // amber
const R: [number, number, number] = [220, 55, 55]     // red
const W: [number, number, number] = [255, 255, 255]   // white

function rgba(c: [number, number, number], a: number) {
  return `rgba(${c[0]},${c[1]},${c[2]},${Math.max(0, Math.min(1, a))})`
}
function lerp(a: number, b: number, t: number) {
  return a + (b - a) * Math.max(0, Math.min(1, t))
}
function roundRect(
  ctx: CanvasRenderingContext2D,
  x: number, y: number, w: number, h: number, r: number
) {
  ctx.beginPath()
  ctx.moveTo(x + r, y)
  ctx.lineTo(x + w - r, y)
  ctx.arcTo(x + w, y, x + w, y + r, r)
  ctx.lineTo(x + w, y + h - r)
  ctx.arcTo(x + w, y + h, x + w - r, y + h, r)
  ctx.lineTo(x + r, y + h)
  ctx.arcTo(x, y + h, x, y + h - r, r)
  ctx.lineTo(x, y + r)
  ctx.arcTo(x, y, x + r, y, r)
  ctx.closePath()
}

// ─── Layout ──────────────────────────────────────────────────────────────────
const CONNS: readonly [number, number][] = [
  [0,1],[0,2],[0,3],[0,4],[0,5],[0,6],  // hub spokes
  [1,2],[2,3],[3,4],[4,5],[5,6],[6,1],  // outer ring
]
const NODE_LABELS = ["leader","node-01","node-02","node-03","node-04","node-05","node-06"]
const BASE_R = [14, 9, 9, 9, 9, 9, 9]

function npos(id: number, w: number, h: number): [number, number] {
  if (id === 0) return [w / 2, h / 2]
  const degs = [-90, -30, 30, 90, 150, 210]
  const rad = (degs[id - 1] * Math.PI) / 180
  const rx = Math.min(w, h) * 0.36
  const ry = rx * 0.7
  return [w / 2 + Math.cos(rad) * rx, h / 2 + Math.sin(rad) * ry]
}

function connHasNode(ci: number, ni: number) {
  return CONNS[ci][0] === ni || CONNS[ci][1] === ni
}

// ─── Types ───────────────────────────────────────────────────────────────────
type Phase = "normal"|"warning"|"explosion"|"degraded"|"respawning"|"recovered"
type NStatus = "healthy"|"warning"|"failed"|"respawning"

interface Packet {
  id: number; from: number; to: number
  t: number; speed: number
  dying: boolean; dAlpha: number
}
interface Spark {
  x: number; y: number; vx: number; vy: number
  life: number; r: number; col: [number,number,number]
  converge?: [number,number] // if set, homers toward this point
}
interface AnimState {
  phase: Phase; phaseStart: number
  failCycle: number; failTarget: number
  nStatus: NStatus[]; nOpacity: number[]
  nScale: number[]; nGlow: number[]
  connFade: number[]
  packets: Packet[]; nextPid: number; lastSpawn: number
  sparks: Spark[]
}

// Cycle through workers so every node gets its turn
const FAIL_ORDER = [1, 4, 2, 5, 3, 6]

const PD: Record<Phase, number> = {
  normal: 5500, warning: 700, explosion: 900,
  degraded: 3000, respawning: 2500, recovered: 1200,
}

// ─── Component ───────────────────────────────────────────────────────────────
export function MeshDataflow() {
  const containerRef = useRef<HTMLDivElement>(null)
  const canvasRef    = useRef<HTMLCanvasElement>(null)
  const rafRef       = useRef<number>(0)
  const sizeRef      = useRef({ w: 800, h: 460 })
  const visRef       = useRef(false)
  const stateRef     = useRef<AnimState>({
    phase: "normal", phaseStart: Date.now(),
    failCycle: 0, failTarget: FAIL_ORDER[0],
    nStatus:  Array(7).fill("healthy"),
    nOpacity: Array(7).fill(1),
    nScale:   Array(7).fill(1),
    nGlow:    Array(7).fill(1),
    connFade: Array(CONNS.length).fill(1),
    packets: [], nextPid: 0, lastSpawn: Date.now(),
    sparks: [],
  })

  const [statusText, setStatusText] = useState("All nodes healthy — data flowing across the cluster")
  const [statusKind, setStatusKind] = useState<"ok"|"warn"|"err"|"boot">("ok")

  useEffect(() => {
    const container = containerRef.current
    const canvas    = canvasRef.current
    if (!container || !canvas) return
    const ctx = canvas.getContext("2d")
    if (!ctx) return

    // ── Resize ──────────────────────────────────────────────────────────────
    function resize() {
      const rect = container!.getBoundingClientRect()
      const dpr  = window.devicePixelRatio || 1
      const w = rect.width
      const h = rect.height
      sizeRef.current = { w, h }
      canvas!.width  = w * dpr
      canvas!.height = h * dpr
      canvas!.style.width  = `${w}px`
      canvas!.style.height = `${h}px`
      ctx!.setTransform(dpr, 0, 0, dpr, 0, 0)
    }
    const ro = new ResizeObserver(resize)
    ro.observe(container)
    resize()

    // ── Intersection observer ───────────────────────────────────────────────
    const io = new IntersectionObserver(
      ([e]) => { visRef.current = e.isIntersecting },
      { threshold: 0.1 }
    )
    io.observe(container)

    // ── Helpers ─────────────────────────────────────────────────────────────
    function spawnPacket(s: AnimState) {
      if (s.packets.length >= 15) return
      const available = CONNS.map((_, ci) => ci).filter(ci => {
        const [a, b] = CONNS[ci]
        return s.nStatus[a] === "healthy" && s.nStatus[b] === "healthy"
      })
      if (!available.length) return
      const ci = available[Math.floor(Math.random() * available.length)]
      const [a, b] = CONNS[ci]
      const [from, to] = Math.random() < 0.5 ? [a, b] : [b, a]
      s.packets.push({
        id: s.nextPid++, from, to,
        t: 0, speed: 0.00032 + Math.random() * 0.00028,
        dying: false, dAlpha: 1,
      })
    }

    function blast(s: AnimState, x: number, y: number) {
      for (let i = 0; i < 32; i++) {
        const angle = Math.random() * Math.PI * 2
        const spd   = 2 + Math.random() * 4.5
        const col   = Math.random() < 0.45 ? R : Math.random() < 0.5 ? A : W
        s.sparks.push({
          x, y,
          vx: Math.cos(angle) * spd,
          vy: Math.sin(angle) * spd,
          life: 1, r: 1.5 + Math.random() * 3, col,
        })
      }
      // Secondary smaller burst
      for (let i = 0; i < 16; i++) {
        const angle = Math.random() * Math.PI * 2
        const spd   = 1 + Math.random() * 2
        s.sparks.push({
          x: x + (Math.random() - 0.5) * 8,
          y: y + (Math.random() - 0.5) * 8,
          vx: Math.cos(angle) * spd, vy: Math.sin(angle) * spd,
          life: 1, r: 1 + Math.random() * 1.5, col: W,
        })
      }
    }

    function materialize(s: AnimState, x: number, y: number) {
      for (let i = 0; i < 22; i++) {
        const angle = (i / 22) * Math.PI * 2 + Math.random() * 0.3
        const d     = 70 + Math.random() * 90
        s.sparks.push({
          x: x + Math.cos(angle) * d,
          y: y + Math.sin(angle) * d,
          vx: 0, vy: 0,
          life: 1, r: 1.5 + Math.random() * 2, col: G,
          converge: [x, y],
        })
      }
    }

    // ── Phase machine ───────────────────────────────────────────────────────
    function tick(s: AnimState, now: number) {
      const elapsed = now - s.phaseStart

      switch (s.phase) {
        case "normal":
          if (elapsed < PD.normal) break
          s.phase = "warning"; s.phaseStart = now
          s.failCycle = (s.failCycle + 1) % FAIL_ORDER.length
          s.failTarget = FAIL_ORDER[s.failCycle]
          s.nStatus[s.failTarget] = "warning"
          setStatusText(`Fault detected — ${NODE_LABELS[s.failTarget]} is unresponsive`)
          setStatusKind("warn")
          break

        case "warning":
          if (elapsed < PD.warning) break
          s.phase = "explosion"; s.phaseStart = now
          setStatusText(`${NODE_LABELS[s.failTarget]} has failed — supervisor activated`)
          setStatusKind("err")
          const { w, h } = sizeRef.current
          const [ex, ey] = npos(s.failTarget, w, h)
          blast(s, ex, ey)
          for (const p of s.packets) {
            if (p.to === s.failTarget || p.from === s.failTarget) p.dying = true
          }
          break

        case "explosion": {
          const t = elapsed / PD.explosion
          s.nOpacity[s.failTarget] = Math.max(0, 1 - t * 2.2)
          s.nScale[s.failTarget]   = t < 0.25 ? lerp(1, 2.2, t / 0.25) : lerp(2.2, 0, (t - 0.25) / 0.75)
          if (elapsed >= PD.explosion) {
            s.phase = "degraded"; s.phaseStart = now
            s.nStatus[s.failTarget]  = "failed"
            s.nOpacity[s.failTarget] = 0
            s.nScale[s.failTarget]   = 0
            for (let ci = 0; ci < CONNS.length; ci++) {
              if (connHasNode(ci, s.failTarget)) s.connFade[ci] = 0.1
            }
            setStatusText("Traffic rerouting automatically — no data lost")
            setStatusKind("warn")
          }
          break
        }

        case "degraded":
          if (elapsed < PD.degraded) break
          s.phase = "respawning"; s.phaseStart = now
          s.nStatus[s.failTarget]  = "respawning"
          s.nOpacity[s.failTarget] = 0
          s.nScale[s.failTarget]   = 0.2
          const { w: w2, h: h2 } = sizeRef.current
          const [rx, ry] = npos(s.failTarget, w2, h2)
          materialize(s, rx, ry)
          setStatusText(`Supervisor restarting ${NODE_LABELS[s.failTarget]}…`)
          setStatusKind("boot")
          break

        case "respawning": {
          const t = Math.min(1, elapsed / PD.respawning)
          const flicker = t < 0.65
            ? (Math.sin(Date.now() / 60) * 0.5 + 0.5) * lerp(0.6, 1, t)
            : 1
          s.nOpacity[s.failTarget] = lerp(0, 1, t) * flicker
          s.nScale[s.failTarget]   = lerp(0.2, 1, Math.pow(t, 0.6))
          s.nGlow[s.failTarget]    = lerp(0, 2.8, t)
          if (elapsed >= PD.respawning) {
            s.phase = "recovered"; s.phaseStart = now
            s.nStatus[s.failTarget]  = "healthy"
            s.nOpacity[s.failTarget] = 1
            s.nScale[s.failTarget]   = 1
            s.nGlow[s.failTarget]    = 2.8
            for (let ci = 0; ci < CONNS.length; ci++) {
              if (connHasNode(ci, s.failTarget)) s.connFade[ci] = 1
            }
            setStatusText(`${NODE_LABELS[s.failTarget]} back online — zero downtime`)
            setStatusKind("ok")
          }
          break
        }

        case "recovered": {
          const t = elapsed / PD.recovered
          s.nGlow[s.failTarget] = lerp(2.8, 1, t)
          if (elapsed >= PD.recovered) {
            s.phase = "normal"; s.phaseStart = now
            s.nGlow[s.failTarget] = 1
            setStatusText("All nodes healthy — data flowing across the cluster")
            setStatusKind("ok")
          }
          break
        }
      }
    }

    // ── Draw frame ──────────────────────────────────────────────────────────
    function frame() {
      rafRef.current = requestAnimationFrame(frame)
      if (!visRef.current) return

      const now  = Date.now()
      const s    = stateRef.current
      const { w, h } = sizeRef.current
      const dt   = 16

      ctx.clearRect(0, 0, w, h)
      tick(s, now)

      // Spawn packets
      if (now - s.lastSpawn > 350) {
        spawnPacket(s)
        s.lastSpawn = now
      }

      // Advance packets
      s.packets = s.packets.filter(p => {
        if (p.dying) { p.dAlpha -= 0.05; return p.dAlpha > 0 }
        p.t += p.speed * dt
        if (p.t >= 1) { p.t = 1; p.dying = true }
        // Kill if endpoint failed
        if (s.nStatus[p.to] === "failed" || s.nStatus[p.from] === "failed") p.dying = true
        return true
      })

      // Advance sparks
      s.sparks = s.sparks.filter(sp => {
        if (sp.converge) {
          const [cx, cy] = sp.converge
          const dx = cx - sp.x; const dy = cy - sp.y
          const d = Math.sqrt(dx * dx + dy * dy)
          if (d < 6) { sp.life = 0; return false }
          sp.x += dx * 0.07 + sp.vx
          sp.y += dy * 0.07 + sp.vy
          sp.vx *= 0.88; sp.vy *= 0.88
        } else {
          sp.x += sp.vx; sp.y += sp.vy
          sp.vy += 0.07; sp.vx *= 0.97
        }
        sp.life -= sp.converge ? 0.012 : 0.018
        return sp.life > 0
      })

      // ── Connections ─────────────────────────────────────────────────────
      for (let ci = 0; ci < CONNS.length; ci++) {
        const [a, b] = CONNS[ci]
        const [ax, ay] = npos(a, w, h)
        const [bx, by] = npos(b, w, h)
        const fade = s.connFade[ci]
        const broken = s.nStatus[a] === "failed" || s.nStatus[b] === "failed"
        const active = s.packets.some(p =>
          !p.dying && ((p.from === a && p.to === b) || (p.from === b && p.to === a))
        )

        const alpha = broken ? 0.05 : (active ? 0.22 : 0.10)
        ctx.beginPath()
        ctx.moveTo(ax, ay)
        ctx.lineTo(bx, by)
        ctx.strokeStyle = rgba(G, alpha * fade)
        ctx.lineWidth   = active ? 1.4 : 0.8
        if (broken) { ctx.setLineDash([4, 6]); ctx.lineDashOffset = (now / 60) % 10 }
        ctx.stroke()
        ctx.setLineDash([])
        ctx.lineDashOffset = 0
      }

      // ── Packets ─────────────────────────────────────────────────────────
      for (const p of s.packets) {
        const [ax, ay] = npos(p.from, w, h)
        const [bx, by] = npos(p.to, w, h)
        const px = lerp(ax, bx, p.t)
        const py = lerp(ay, by, p.t)
        const alpha = p.dying ? p.dAlpha * 0.85 : 0.85

        // Glow
        const grd = ctx.createRadialGradient(px, py, 0, px, py, 11)
        grd.addColorStop(0, rgba(G, 0.3 * alpha))
        grd.addColorStop(1, rgba(G, 0))
        ctx.beginPath(); ctx.arc(px, py, 11, 0, Math.PI * 2)
        ctx.fillStyle = grd; ctx.fill()

        // Tail
        if (!p.dying && p.t > 0.05) {
          const tp = Math.max(0, p.t - 0.08)
          const tx = lerp(ax, bx, tp), ty = lerp(ay, by, tp)
          ctx.beginPath(); ctx.moveTo(tx, ty); ctx.lineTo(px, py)
          ctx.strokeStyle = rgba(G, 0.28 * alpha)
          ctx.lineWidth = 1.8; ctx.stroke()
        }

        // Core dot
        ctx.beginPath(); ctx.arc(px, py, 2.8, 0, Math.PI * 2)
        ctx.fillStyle = rgba(G, alpha); ctx.fill()
      }

      // ── Sparks ──────────────────────────────────────────────────────────
      for (const sp of s.sparks) {
        const a = sp.life * (sp.converge ? 0.8 : 0.9)
        const r = sp.converge ? sp.r : sp.r * (0.4 + sp.life * 0.6)
        ctx.beginPath(); ctx.arc(sp.x, sp.y, r, 0, Math.PI * 2)
        ctx.fillStyle = rgba(sp.col, a); ctx.fill()
        // Tiny glow on bigger sparks
        if (r > 2 && !sp.converge) {
          const g2 = ctx.createRadialGradient(sp.x, sp.y, 0, sp.x, sp.y, r * 2.5)
          g2.addColorStop(0, rgba(sp.col, a * 0.25)); g2.addColorStop(1, rgba(sp.col, 0))
          ctx.beginPath(); ctx.arc(sp.x, sp.y, r * 2.5, 0, Math.PI * 2)
          ctx.fillStyle = g2; ctx.fill()
        }
      }

      // ── Nodes ───────────────────────────────────────────────────────────
      for (let i = 0; i < 7; i++) {
        const [nx, ny] = npos(i, w, h)
        const status  = s.nStatus[i]
        const opacity = s.nOpacity[i]
        const scale   = s.nScale[i]
        const glow    = s.nGlow[i]

        if (status === "failed" && s.phase !== "explosion") continue
        if (opacity < 0.01) continue

        const color: [number,number,number] = status === "warning" ? A : G
        const baseR = BASE_R[i]
        const r     = baseR * scale

        // Pulse
        const pFreq = status === "warning" ? 0.007 : status === "respawning" ? 0.009 : 0.0022
        const pAmp  = status === "warning" ? 0.35  : status === "respawning" ? 0.2   : 0.1
        const pulse = 1 + Math.sin(now * pFreq + i * 1.4) * pAmp
        const pr    = r * pulse

        // Outer glow halo
        const glowR = baseR * 4 * glow
        const halo  = ctx.createRadialGradient(nx, ny, pr * 0.3, nx, ny, glowR)
        halo.addColorStop(0,   rgba(color, 0.22 * opacity * glow))
        halo.addColorStop(0.4, rgba(color, 0.09 * opacity * glow))
        halo.addColorStop(1,   rgba(color, 0))
        ctx.beginPath(); ctx.arc(nx, ny, glowR, 0, Math.PI * 2)
        ctx.fillStyle = halo; ctx.fill()

        // Second wider ambient for leader
        if (i === 0) {
          const ambient = ctx.createRadialGradient(nx, ny, 0, nx, ny, glowR * 1.8)
          ambient.addColorStop(0,   rgba(G, 0.06 * opacity))
          ambient.addColorStop(1,   rgba(G, 0))
          ctx.beginPath(); ctx.arc(nx, ny, glowR * 1.8, 0, Math.PI * 2)
          ctx.fillStyle = ambient; ctx.fill()
        }

        // Ring (respawning: dashed)
        if (status === "respawning") {
          ctx.beginPath(); ctx.arc(nx, ny, pr + 5, 0, Math.PI * 2)
          ctx.strokeStyle = rgba(G, opacity * 0.45)
          ctx.lineWidth = 1.5
          ctx.setLineDash([3, 5])
          ctx.lineDashOffset = -(now / 80) % 8
          ctx.stroke(); ctx.setLineDash([]); ctx.lineDashOffset = 0
        }

        // Node body gradient
        const body = ctx.createRadialGradient(nx - pr * 0.3, ny - pr * 0.3, 0, nx, ny, pr)
        if (i === 0) {
          body.addColorStop(0, rgba(W, opacity * 0.95))
          body.addColorStop(0.5, rgba(G, opacity * 0.9))
          body.addColorStop(1, rgba(G, opacity * 0.65))
        } else {
          body.addColorStop(0, rgba(color, opacity * 0.95))
          body.addColorStop(1, rgba(color, opacity * 0.55))
        }
        ctx.beginPath(); ctx.arc(nx, ny, pr, 0, Math.PI * 2)
        ctx.fillStyle = body; ctx.fill()

        // Outer ring
        if (status !== "failed") {
          ctx.beginPath(); ctx.arc(nx, ny, pr + 2.5, 0, Math.PI * 2)
          ctx.strokeStyle = rgba(color, opacity * (status === "warning" ? 0.7 : 0.22))
          ctx.lineWidth = status === "warning" ? 1.5 : 0.8
          ctx.stroke()
        }

        // Label
        if (opacity > 0.15) {
          const lsize = i === 0 ? 10 : 9.5
          ctx.font = `${i === 0 ? 600 : 400} ${lsize}px ui-monospace,monospace`
          ctx.fillStyle = rgba(W, opacity * (i === 0 ? 0.6 : 0.38))
          ctx.textAlign = "center"
          ctx.textBaseline = "top"
          ctx.fillText(NODE_LABELS[i], nx, ny + pr + 7)
        }

        // Status chip
        if (status !== "healthy" && opacity > 0.08) {
          const chipTxt = status === "warning" ? "● degrading"
            : status === "failed"    ? "offline"
            : "restarting…"
          const chipCol: [number,number,number] = status === "warning" ? A : status === "failed" ? R : G
          ctx.font = "bold 9px ui-monospace,monospace"
          const tw = ctx.measureText(chipTxt).width
          const cw = tw + 12; const ch = 17
          const cx2 = nx - cw / 2; const cy2 = ny - pr - ch - 10
          roundRect(ctx, cx2, cy2, cw, ch, 4)
          ctx.fillStyle   = rgba(chipCol, 0.13 * opacity); ctx.fill()
          roundRect(ctx, cx2, cy2, cw, ch, 4)
          ctx.strokeStyle = rgba(chipCol, 0.45 * opacity); ctx.lineWidth = 0.8; ctx.stroke()
          ctx.fillStyle   = rgba(chipCol, opacity * 0.9)
          ctx.textAlign = "center"; ctx.textBaseline = "middle"
          ctx.fillText(chipTxt, nx, cy2 + ch / 2)
        }
      }
    }

    rafRef.current = requestAnimationFrame(frame)

    return () => {
      cancelAnimationFrame(rafRef.current)
      ro.disconnect()
      io.disconnect()
    }
  }, [])

  const dotCls = {
    ok:   "bg-accent",
    warn: "bg-amber-400 animate-pulse",
    err:  "bg-red-400 animate-pulse",
    boot: "bg-accent animate-pulse",
  }[statusKind]

  const textCls = {
    ok:   "text-accent",
    warn: "text-amber-400",
    err:  "text-red-400",
    boot: "text-accent",
  }[statusKind]

  const legend = [
    { dot: "bg-accent",       label: "Healthy node"  },
    { dot: "bg-amber-400",    label: "Warning"        },
    { dot: "bg-red-400",      label: "Failed"         },
    { dot: "bg-accent/40",    label: "Respawning"     },
    { dot: "bg-accent rounded-full scale-75", label: "Data packet" },
  ]

  return (
    <section className="relative border-t border-border py-16 sm:py-24 overflow-hidden">
      {/* Ambient glow */}
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_70%_55%_at_50%_50%,rgba(89,193,132,0.035),transparent)]" />

      <div className="relative mx-auto max-w-5xl px-4 sm:px-6">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5 }}
          className="mb-8 text-center"
        >
          <p className="mb-3 text-sm font-mono uppercase tracking-wider text-accent">Live simulation</p>
          <h2 className="mb-3 text-3xl font-bold tracking-tight sm:text-4xl">
            Fault tolerance. In real time.
          </h2>
          <p className="mx-auto max-w-lg text-muted-foreground text-pretty">
            Watch what happens when a node fails. Traffic reroutes, the supervisor
            restarts it, and nothing is lost.
          </p>
        </motion.div>

        {/* Canvas wrapper */}
        <motion.div
          initial={{ opacity: 0, y: 28 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6, delay: 0.15 }}
          ref={containerRef}
          className="relative w-full overflow-hidden rounded-2xl border border-border bg-[#060e0a] shadow-[0_0_0_1px_rgba(89,193,132,0.06),0_24px_80px_rgba(0,0,0,0.55)]"
          style={{ height: "clamp(300px, 44vw, 520px)" }}
        >
          {/* Corner badge */}
          <div className="absolute left-4 top-3 z-10 flex select-none items-center gap-1.5 text-[10px] font-mono text-muted-foreground/40">
            <span className="h-1.5 w-1.5 rounded-full bg-accent/60 animate-pulse" />
            mesh cluster · live
          </div>

          <canvas ref={canvasRef} className="absolute inset-0" />
        </motion.div>

        {/* Status bar */}
        <motion.div
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true }}
          transition={{ duration: 0.4, delay: 0.3 }}
          className="mt-3 flex items-center gap-3 rounded-xl border border-border bg-card/50 px-4 py-3"
        >
          <div className={`h-2 w-2 shrink-0 rounded-full ${dotCls}`} />
          <p className={`text-sm font-mono transition-colors duration-300 ${textCls}`}>
            {statusText}
          </p>
        </motion.div>

        {/* Legend */}
        <div className="mt-4 flex flex-wrap items-center justify-center gap-x-5 gap-y-2.5">
          {legend.map(item => (
            <div key={item.label} className="flex items-center gap-2">
              <div className={`h-2.5 w-2.5 rounded-full ${item.dot}`} />
              <span className="text-xs text-muted-foreground/60">{item.label}</span>
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}
