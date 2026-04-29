// Claw Desktop - Agent 3D可视化 - Three.js渲染Agent节点关系图、连接线和动画效果
import React, { useState, useMemo, useCallback, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { Canvas, useFrame } from '@react-three/fiber'
import { OrbitControls, Line } from '@react-three/drei'
import * as THREE from 'three'
import AgentNode3D from './AgentNode3D'
import { getVisualStyleForAgent, getAgentColor } from './AgentNode3D'
import { useAgentStatus } from '../../hooks/useAgentStatus'
import type { AgentVisualData, AgentConnection } from '../../hooks/useAgentStatus'

// ==================== 协作连线组件 ====================

const ConnectionLine: React.FC<{
  from: [number, number, number]
  to: [number, number, number]
  strength: number
}> = React.memo(({ from, to, strength }) => {
  const groupRef = useRef<THREE.Group>(null!)
  const timeRef = useRef(Math.random() * 100)

  const { mainPoints, particlePositions } = useMemo(() => {
    const mid: [number, number, number] = [
      (from[0] + to[0]) / 2,
      (from[1] + to[1]) / 2 + 1.5,
      (from[2] + to[2]) / 2,
    ]
    const curve = new THREE.QuadraticBezierCurve3(
      new THREE.Vector3(...from),
      new THREE.Vector3(...mid),
      new THREE.Vector3(...to)
    )
    const mainPts = curve.getPoints(40).map(p => [p.x, p.y, p.z] as [number, number, number])

    // 粒子沿曲线分布
    const pCount = 6
    const pPositions = new Float32Array(pCount * 3)
    for (let i = 0; i < pCount; i++) {
      const t = i / (pCount - 1)
      const pt = curve.getPoint(t)
      pPositions[i * 3] = pt.x
      pPositions[i * 3 + 1] = pt.y
      pPositions[i * 3 + 2] = pt.z
    }

    return { mainPoints: mainPts, particlePositions: pPositions }
  }, [from, to])

  useFrame((_, delta) => {
    timeRef.current += delta * 0.5 * strength
    if (groupRef.current) {
      // 微弱脉动
      const pulse = 0.85 + 0.15 * Math.sin(timeRef.current * 2)
      groupRef.current.scale.setScalar(pulse)
    }
  })

  return (
    <group ref={groupRef}>
      {/* 主连线 */}
      <Line
        points={mainPoints}
        color="#748ffc"
        lineWidth={2}
        transparent
        opacity={0.25 * strength}
      />
      {/* 亮色核心线 */}
      <Line
        points={mainPoints}
        color="#91a7ff"
        lineWidth={0.8}
        transparent
        opacity={0.5 * strength}
      />
      {/* 流动粒子 */}
      <points>
        <bufferGeometry>
          <bufferAttribute
            attach="attributes-position"
            args={[particlePositions, 3]}
            count={6}
          />
        </bufferGeometry>
        <pointsMaterial
          color="#bac8ff"
          size={0.06}
          transparent
          opacity={0.6 * strength}
          sizeAttenuation
        />
      </points>
    </group>
  )
})
ConnectionLine.displayName = 'ConnectionLine'

// ==================== 网格地面组件 ====================

const GridFloor: React.FC = React.memo(() => {
  const gridRef = useRef<THREE.Group>(null!)

  const gridLines = useMemo(() => {
    const lines: Array<{ points: [number, number, number][]; color: string; opacity: number }> = []
    const size = 30
    const divisions = 30
    const step = size / divisions
    const half = size / 2

    // 水平线
    for (let i = 0; i <= divisions; i++) {
      const pos = -half + i * step
      const dist = Math.abs(pos) / half
      const opacity = 0.08 * (1 - dist * 0.7)
      lines.push({
        points: [[-half, 0, pos], [half, 0, pos]],
        color: '#1a3a5c',
        opacity,
      })
    }
    // 垂直线
    for (let i = 0; i <= divisions; i++) {
      const pos = -half + i * step
      const dist = Math.abs(pos) / half
      const opacity = 0.08 * (1 - dist * 0.7)
      lines.push({
        points: [[pos, 0, -half], [pos, 0, half]],
        color: '#1a3a5c',
        opacity,
      })
    }
    return lines
  }, [])

  return (
    <group ref={gridRef} position={[0, -5, 0]}>
      {gridLines.map((line, i) => (
        <Line
          key={`grid-${i}`}
          points={line.points}
          color={line.color}
          lineWidth={0.5}
          transparent
          opacity={line.opacity}
        />
      ))}
    </group>
  )
})
GridFloor.displayName = 'GridFloor'

// ==================== 背景粒子系统 ====================

const BackgroundParticles: React.FC = React.memo(() => {
  const count = 500
  const meshRef = useRef<THREE.Points>(null!)

  const { positions, colors } = useMemo(() => {
    const pos = new Float32Array(count * 3)
    const col = new Float32Array(count * 3)
    const palette = [
      new THREE.Color('#4c6ef5'),
      new THREE.Color('#748ffc'),
      new THREE.Color('#da77f2'),
      new THREE.Color('#38d9a9'),
    ]
    for (let i = 0; i < count; i++) {
      // 球形分布
      const radius = 8 + Math.random() * 25
      const theta = Math.random() * Math.PI * 2
      const phi = Math.acos(2 * Math.random() - 1)
      pos[i * 3] = radius * Math.sin(phi) * Math.cos(theta)
      pos[i * 3 + 1] = radius * Math.sin(phi) * Math.sin(theta)
      pos[i * 3 + 2] = radius * Math.cos(phi)

      const c = palette[Math.floor(Math.random() * palette.length)]
      col[i * 3] = c.r
      col[i * 3 + 1] = c.g
      col[i * 3 + 2] = c.b
    }
    return { positions: pos, colors: col }
  }, [])

  useFrame((_, delta) => {
    if (meshRef.current) {
      meshRef.current.rotation.y += delta * 0.015
      meshRef.current.rotation.x += delta * 0.005
    }
  })

  return (
    <points ref={meshRef}>
      <bufferGeometry>
        <bufferAttribute attach="attributes-position" args={[positions, 3]} count={count} />
        <bufferAttribute attach="attributes-color" args={[colors, 3]} count={count} />
      </bufferGeometry>
      <pointsMaterial
        size={0.04}
        transparent
        opacity={0.6}
        vertexColors
        sizeAttenuation
        blending={THREE.AdditiveBlending}
        depthWrite={false}
      />
    </points>
  )
})
BackgroundParticles.displayName = 'BackgroundParticles'

// ==================== 浮动光斑组件 ====================

const FloatingOrbs: React.FC = React.memo(() => {
  const orbCount = 8
  const orbsRef = useRef<THREE.Group>(null!)
  const timeRef = useRef(0)

  const orbData = useMemo(() => {
    return Array.from({ length: orbCount }, () => ({
      position: [
        (Math.random() - 0.5) * 20,
        (Math.random() - 0.5) * 12,
        (Math.random() - 0.5) * 15,
      ] as [number, number, number],
      color: ['#4c6ef5', '#748ffc', '#da77f2', '#38d9a9', '#ffd43b'][Math.floor(Math.random() * 5)],
      speed: 0.3 + Math.random() * 0.5,
      phase: Math.random() * Math.PI * 2,
      size: 0.08 + Math.random() * 0.15,
    }))
  }, [])

  useFrame((_, delta) => {
    timeRef.current += delta
  })

  return (
    <group ref={orbsRef}>
      {orbData.map((orb, i) => (
        <mesh key={i} position={orb.position}>
          <sphereGeometry args={[orb.size, 16, 16]} />
          <meshBasicMaterial
            color={orb.color}
            transparent
            opacity={0.15}
          />
        </mesh>
      ))}
    </group>
  )
})
FloatingOrbs.displayName = 'FloatingOrbs'

// ==================== 类别区域标识 ====================

const CategoryZone: React.FC<{
  position: [number, number, number]
  color: string
  label: string
}> = React.memo(({ position, color, label }) => {
  const ringRef = useRef<THREE.Mesh>(null!)

  useFrame((_, delta) => {
    if (ringRef.current) {
      ringRef.current.rotation.z += delta * 0.15
    }
  })

  return (
    <group position={position}>
      {/* 底部区域环 */}
      <mesh ref={ringRef} rotation={[Math.PI / 2, 0, 0]} position={[0, -0.8, 0]}>
        <torusGeometry args={[1.8, 0.008, 8, 64]} />
        <meshBasicMaterial color={color} transparent opacity={0.25} />
      </mesh>
      {/* 区域标签 */}
      {/* 注：使用简单平面代替 Text，避免字体加载延迟 */}
    </group>
  )
})
CategoryZone.displayName = 'CategoryZone'

// ==================== Agent Detail Panel ====================

function getStatusLabels(t: (key: string) => string) {
  return {
    idle: { text: t('panels.visualization.statusIdle'), color: '#4c6ef5', icon: '○' },
    running: { text: t('panels.visualization.statusRunning'), color: '#748ffc', icon: '●' },
    completed: { text: t('panels.visualization.statusCompleted'), color: '#51cf66', icon: '✓' },
    failed: { text: t('panels.visualization.statusFailed'), color: '#e94560', icon: '✕' },
    pending: { text: t('panels.visualization.statusPending'), color: '#8892a0', icon: '◔' },
    waiting_input: { text: t('panels.visualization.statusWaitingInput'), color: '#fcc419', icon: '◷' },
    timeout: { text: t('panels.visualization.statusTimeout'), color: '#ff6b6b', icon: '⊘' },
  }
}

function getCategoryLabels(t: (key: string) => string) {
  return {
    search: { text: t('panels.visualization.categorySearch'), color: '#38d9a9' },
    code: { text: t('panels.visualization.categoryCode'), color: '#69db7c' },
    analysis: { text: t('panels.visualization.categoryAnalysis'), color: '#da77f2' },
    creative: { text: t('panels.visualization.categoryCreative'), color: '#ffd43b' },
    general: { text: t('panels.visualization.categoryGeneral'), color: '#748ffc' },
    custom: { text: t('panels.visualization.categoryCustom'), color: '#ff922b' },
  }
}

const EXTENDED_STYLE_KEYS: Record<string, string> = {
  hexPrism: 'styleHexPrism',
  doubleTorus: 'styleDoubleTorus',
  crystal: 'styleCrystal',
  starTetra: 'styleStarTetra',
  spiralHelix: 'styleSpiralHelix',
  diamond: 'styleDiamond',
  pyramid: 'stylePyramid',
  mobius: 'styleMobius',
  fragmentedCube: 'styleFragmentedCube',
  lotus: 'styleLotus',
}

const AgentDetailPanel: React.FC<{
  agent: AgentVisualData
  onClose: () => void
}> = ({ agent, onClose }) => {
  const { t } = useTranslation()
  const statusLabels = getStatusLabels(t)
  const categoryLabels = getCategoryLabels(t)
  const statusInfo = statusLabels[agent.status] || statusLabels.idle
  const categoryInfo = categoryLabels[agent.category] || categoryLabels.custom
  const visualStyle = getVisualStyleForAgent(agent.id, agent.category)
  const styleColor = getAgentColor(agent.id, agent.category)
  const styleName = agent.category === 'custom' ? (t(`panels.visualization.${EXTENDED_STYLE_KEYS[visualStyle]}`) || t('panels.visualization.categoryCustom')) : null

  return (
    <div className="agent-detail-panel absolute right-4 top-4 w-80 rounded-2xl border border-[#1a3a5c]/60 bg-gradient-to-br from-[#0d1b2a]/95 via-[#16213e]/95 to-[#1a1a2e]/95 backdrop-blur-2xl shadow-2xl shadow-black/50 z-10 overflow-hidden">
      {/* 顶部渐变条 */}
      <div className="h-1 w-full" style={{ background: `linear-gradient(90deg, ${styleColor}80, ${statusInfo.color}80)` }} />

      {/* 头部 */}
      <div className="flex items-center justify-between px-5 py-3.5 border-b border-[#1a3a5c]/30">
        <div className="flex items-center gap-3">
          <div
            className="w-9 h-9 rounded-xl flex items-center justify-center text-lg"
            style={{ backgroundColor: `${styleColor}20`, border: `1px solid ${styleColor}30` }}
          >
            {agent.icon}
          </div>
          <div>
            <h3 className="text-[#e0e0e0] font-semibold text-sm leading-tight">{agent.name}</h3>
            <span
              className="text-[10px] font-medium tracking-wider uppercase"
              style={{ color: styleColor }}
            >
              {styleName ? `${categoryInfo.text} · ${styleName}` : categoryInfo.text}
            </span>
          </div>
        </div>
        <button
          onClick={onClose}
          className="w-7 h-7 flex items-center justify-center rounded-lg hover:bg-[#1a3a5c]/50 text-[#8892a0] hover:text-[#e0e0e0] transition-colors text-xs"
        >
          ✕
        </button>
      </div>

      {/* 状态 */}
      <div className="px-5 py-2.5 flex items-center gap-2.5">
        <div className="flex items-center gap-1.5">
          <span
            className="w-2 h-2 rounded-full animate-pulse"
            style={{ backgroundColor: statusInfo.color, boxShadow: `0 0 6px ${statusInfo.color}60` }}
          />
          <span className="text-xs font-medium" style={{ color: statusInfo.color }}>
            {statusInfo.icon} {statusInfo.text}
          </span>
        </div>
        {agent.activeTaskCount > 0 && (
              <span className="ml-auto text-[10px] text-[#8892a0] bg-[#1a3a5c]/40 px-2 py-0.5 rounded-md border border-[#1a3a5c]/30">
                {agent.activeTaskCount} {t('panels.visualization.tasks')}
              </span>
            )}
      </div>

      {/* 描述 */}
      <div className="px-5 py-2">
        <p className="text-[11px] text-[#8892a0] leading-relaxed">{agent.description}</p>
      </div>

      {/* 能力标签 */}
      {agent.capabilities.length > 0 && (
        <div className="px-5 py-2">
          <div className="text-[9px] text-[#8892a0]/70 mb-1.5 uppercase tracking-widest font-medium">{t('panels.visualization.capabilities')}</div>
          <div className="flex flex-wrap gap-1">
            {agent.capabilities.map(cap => (
              <span
                key={cap}
                className="text-[10px] px-2 py-0.5 rounded-md border"
                style={{
                  color: styleColor,
                  backgroundColor: `${styleColor}10`,
                  borderColor: `${styleColor}20`,
                }}
              >
                {cap}
              </span>
            ))}
          </div>
        </div>
      )}

      {/* 当前任务 */}
      {agent.currentTask && (
        <div className="px-5 py-2.5 border-t border-[#1a3a5c]/20">
          <div className="text-[9px] text-[#8892a0]/70 mb-1 uppercase tracking-widest font-medium">{t('panels.visualization.currentTask')}</div>
          <p className="text-[11px] text-[#e0e0e0] bg-[#050510]/60 rounded-lg px-3 py-2 leading-relaxed border border-[#1a3a5c]/20">
            {agent.currentTask}
          </p>
        </div>
      )}

      {/* 协作信息 */}
      {agent.isCollaborating && agent.collaboratingWith.length > 0 && (
        <div className="px-5 py-2.5 border-t border-[#1a3a5c]/20">
          <div className="text-[9px] text-[#8892a0]/70 mb-1.5 uppercase tracking-widest font-medium">{t('panels.visualization.collaborating')}</div>
          <div className="flex flex-wrap gap-1">
            {agent.collaboratingWith.map(id => (
              <span
                key={id}
                className="text-[10px] px-2 py-0.5 rounded-md bg-[#ffd43b]/8 text-[#ffd43b] border border-[#ffd43b]/15"
              >
                {id}
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

// ==================== 操作提示组件 ====================

const ControlsHint: React.FC = React.memo(() => (
  <div className="absolute left-4 bottom-4 z-10 select-none pointer-events-none">
    <div className="text-[10px] text-[#8892a0]/40 flex items-center gap-3">
      <span>🖱 拖拽旋转</span>
      <span className="text-[#1a3a5c]">|</span>
      <span>⚙ 滚轮缩放</span>
      <span className="text-[#1a3a5c]">|</span>
      <span>👆 点击选中</span>
    </div>
  </div>
))
ControlsHint.displayName = 'ControlsHint'

// ==================== 场景内3D组件 ====================

interface SceneContentProps {
  visualData: AgentVisualData[]
  connections: AgentConnection[]
  selectedAgentId: string | null
  onSelectAgent: (id: string) => void
}

const SceneContent: React.FC<SceneContentProps> = React.memo(({
  visualData,
  connections,
  selectedAgentId,
  onSelectAgent,
}) => {
  const { t } = useTranslation()
  const positionMap = useMemo(() => {
    const map = new Map<string, [number, number, number]>()
    for (const d of visualData) {
      map.set(d.id, d.position)
    }
    return map
  }, [visualData])

  const categoryZones = useMemo(() => {
    const CATEGORY_ZONE_CONFIG: Record<string, { position: [number, number, number]; color: string; labelKey: string }> = {
      search: { position: [-4, 1.5, 0], color: '#38d9a9', labelKey: 'categorySearch' },
      code: { position: [4, 1.5, 0], color: '#69db7c', labelKey: 'categoryCode' },
      analysis: { position: [0, 0, 1], color: '#da77f2', labelKey: 'categoryAnalysis' },
      creative: { position: [-3, -2, -1], color: '#ffd43b', labelKey: 'categoryCreative' },
      general: { position: [3, -2, -1], color: '#748ffc', labelKey: 'categoryGeneral' },
      custom: { position: [0, -3.5, 1], color: '#ff922b', labelKey: 'categoryCustom' },
    }
    const seen = new Set<string>()
    return visualData
      .filter(d => {
        if (seen.has(d.category)) return false
        seen.add(d.category)
        return true
      })
      .map(d => {
        const cfg = CATEGORY_ZONE_CONFIG[d.category]
        if (!cfg) return null
        const color = d.category === 'custom' ? getAgentColor(d.id, d.category as unknown as import('../../multiagent/types').AgentCategory) : cfg.color
        return { position: cfg.position, color, label: t(`panels.visualization.${cfg.labelKey}`) }
      })
      .filter(Boolean) as Array<{ position: [number, number, number]; color: string; label: string }>
  }, [visualData, t])

  return (
    <>
      {/* 灯光系统 */}
      <ambientLight intensity={0.15} />
      <directionalLight
        position={[10, 15, 8]}
        intensity={0.6}
        color="#91a7ff"
      />
      <pointLight position={[5, 8, 5]} intensity={1.0} color="#748ffc" distance={30} decay={2} />
      <pointLight position={[-8, 4, -6]} intensity={0.6} color="#4c6ef5" distance={25} decay={2} />
      <pointLight position={[0, -3, 8]} intensity={0.4} color="#da77f2" distance={20} decay={2} />
      <pointLight position={[-5, 6, -8]} intensity={0.3} color="#38d9a9" distance={25} decay={2} />
      {/* 底部补光 */}
      <pointLight position={[0, -8, 0]} intensity={0.2} color="#1a3a5c" distance={30} decay={2} />

      {/* 环境贴图替代 - 半球光 */}
      <hemisphereLight args={['#1a3a5c', '#050510', 0.3]} />

      {/* 控制器 */}
      <OrbitControls
        enableDamping
        dampingFactor={0.06}
        minDistance={5}
        maxDistance={22}
        enablePan
        panSpeed={0.6}
        rotateSpeed={0.5}
        autoRotate={false}
        maxPolarAngle={Math.PI * 0.85}
        minPolarAngle={Math.PI * 0.1}
      />

      {/* 网格地面 */}
      <GridFloor />

      {/* 背景粒子 */}
      <BackgroundParticles />

      {/* 浮动光斑 */}
      <FloatingOrbs />

      {/* 类别区域标识 */}
      {categoryZones.map((zone, i) => (
        <CategoryZone
          key={`zone-${i}`}
          position={zone.position}
          color={zone.color}
          label={zone.label}
        />
      ))}

      {/* Agent 节点 */}
      {visualData.map(agent => (
        <AgentNode3D
          key={agent.id}
          data={agent}
          isSelected={selectedAgentId === agent.id}
          onSelect={onSelectAgent}
        />
      ))}

      {/* 协作连线 */}
      {connections.map(conn => {
        const from = positionMap.get(conn.fromId)
        const to = positionMap.get(conn.toId)
        if (!from || !to) return null
        return (
          <ConnectionLine
            key={`${conn.fromId}-${conn.toId}`}
            from={from}
            to={to}
            strength={conn.strength}
          />
        )
      })}
    </>
  )
})
SceneContent.displayName = 'SceneContent'

// ==================== 主组件 ====================

interface AgentVisualization3DProps {
  agents: Array<{ id: string; displayName: string; description?: string }>
  convState: Record<string, { multiAgentMessages: any[] }>
  isActive: boolean
}

const AgentVisualization3D: React.FC<AgentVisualization3DProps> = ({
  agents,
  convState,
  isActive,
}) => {
  const [selectedAgentId, setSelectedAgentId] = useState<string | null>(null)

  const { visualData, connections } = useAgentStatus({
    agents,
    convState,
    isActive,
  })

  const selectedAgent = useMemo(
    () => visualData.find(a => a.id === selectedAgentId) || null,
    [visualData, selectedAgentId]
  )

  const handleSelectAgent = useCallback((id: string) => {
    setSelectedAgentId(prev => (prev === id ? null : id))
  }, [])

  const handleCloseDetail = useCallback(() => {
    setSelectedAgentId(null)
  }, [])

  const runningCount = visualData.filter(a => a.status === 'running').length

  return (
    <div className="relative w-full h-full bg-[#050510]">
      {/* 3D Canvas */}
      <Canvas
        camera={{ position: [0, 4, 14], fov: 45, near: 0.1, far: 100 }}
        gl={{
          antialias: true,
          alpha: false,
          powerPreference: 'high-performance',
          toneMapping: THREE.ACESFilmicToneMapping,
          toneMappingExposure: 1.2,
        }}
        dpr={[1, 2]}
        style={{ width: '100%', height: '100%' }}
        onPointerMissed={handleCloseDetail}
      >
        <color attach="background" args={['#050510']} />
        <fog attach="fog" args={['#050510', 16, 32]} />
        <SceneContent
          visualData={visualData}
          connections={connections}
          selectedAgentId={selectedAgentId}
          onSelectAgent={handleSelectAgent}
        />
      </Canvas>

      {/* 顶部渐变遮罩 */}
      <div className="absolute inset-x-0 top-0 h-20 bg-gradient-to-b from-[#050510]/60 to-transparent pointer-events-none z-[5]" />
      {/* 底部渐变遮罩 */}
      <div className="absolute inset-x-0 bottom-0 h-16 bg-gradient-to-t from-[#050510]/40 to-transparent pointer-events-none z-[5]" />

      {/* Agent 详情浮层 */}
      {selectedAgent && (
        <AgentDetailPanel agent={selectedAgent} onClose={handleCloseDetail} />
      )}

      {/* 操作提示 */}
      <ControlsHint />

      {/* 左上角统计信息 */}
      <div className="absolute left-4 top-4 z-10 select-none pointer-events-none">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <div className="w-1.5 h-1.5 rounded-full bg-[#4c6ef5]/60" />
            <span className="text-[10px] text-[#8892a0]/50 font-mono">AGENTS {visualData.length}</span>
          </div>
          <div className="flex items-center gap-2">
            <div className={`w-1.5 h-1.5 rounded-full ${runningCount > 0 ? 'bg-[#748ffc]/80 animate-pulse' : 'bg-[#8892a0]/30'}`} />
            <span className="text-[10px] text-[#8892a0]/50 font-mono">ACTIVE {runningCount}</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-1.5 h-1.5 rounded-full bg-[#ffd43b]/50" />
            <span className="text-[10px] text-[#8892a0]/50 font-mono">LINKS {connections.length}</span>
          </div>
        </div>
      </div>

      {/* 右下角标题 */}
      <div className="absolute right-4 bottom-4 z-10 select-none pointer-events-none">
        <div className="text-[10px] text-[#8892a0]/25 font-mono tracking-widest">
          AGENT NETWORK v2.0
        </div>
      </div>
    </div>
  )
}

export default AgentVisualization3D
