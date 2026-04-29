// Claw Desktop - Agent 3D节点 - 单个Agent的3D渲染组件（球体+标签+脉冲动画）
import React, { useRef, useMemo } from 'react'
import { useFrame } from '@react-three/fiber'
import { Billboard, Text } from '@react-three/drei'
import * as THREE from 'three'
import type { AgentVisualData, AgentVisualStatus } from '../../hooks/useAgentStatus'
import { AgentCategory } from '../../multiagent/types'

// ==================== 常量 ====================

/** 状态 -> 颜色映射 */
const STATUS_COLORS: Record<AgentVisualStatus, string> = {
  idle: '#4c6ef5',
  running: '#748ffc',
  completed: '#51cf66',
  failed: '#e94560',
  pending: '#4c6ef580',
  waiting_input: '#fcc419',
  timeout: '#ff6b6b',
}

/** 类别 -> 主色映射 */
const CATEGORY_COLORS: Record<AgentCategory, string> = {
  [AgentCategory.SEARCH]: '#38d9a9',
  [AgentCategory.CODE]: '#69db7c',
  [AgentCategory.ANALYSIS]: '#da77f2',
  [AgentCategory.CREATIVE]: '#ffd43b',
  [AgentCategory.GENERAL]: '#748ffc',
  [AgentCategory.CUSTOM]: '#ff922b',
}

/** 类别 -> 辅色映射（用于内层/装饰） */
const CATEGORY_ACCENT: Record<AgentCategory, string> = {
  [AgentCategory.SEARCH]: '#20c997',
  [AgentCategory.CODE]: '#37b24d',
  [AgentCategory.ANALYSIS]: '#be4bdb',
  [AgentCategory.CREATIVE]: '#fab005',
  [AgentCategory.GENERAL]: '#5c7cfa',
  [AgentCategory.CUSTOM]: '#f76707',
}

/** 状态 -> 发光强度 */
const STATUS_EMISSIVE_INTENSITY: Record<AgentVisualStatus, number> = {
  idle: 0.2,
  running: 1.0,
  completed: 0.6,
  failed: 0.8,
  pending: 0.05,
  waiting_input: 0.6,
  timeout: 0.7,
}

/** 类别 -> 旋转速度倍率 */
const CATEGORY_ROTATION_SPEED: Record<AgentCategory, number> = {
  [AgentCategory.SEARCH]: 0.3,
  [AgentCategory.CODE]: 0.5,
  [AgentCategory.ANALYSIS]: 0.2,
  [AgentCategory.CREATIVE]: 0.8,
  [AgentCategory.GENERAL]: 0.15,
  [AgentCategory.CUSTOM]: 0.4,
}

// ==================== 扩展视觉效果类型 ====================

/** 视觉风格标识 — 6个内置 + 10种自定义扩展 */
export type VisualStyleType =
  // 内置类别
  | 'octahedron'     // 搜索：八面体
  | 'icosahedron'    // 代码：二十面体
  | 'dodecahedron'   // 分析：十二面体
  | 'torusKnot'      // 创意：环面纽结
  | 'sphere'         // 通用：球体
  | 'cone'           // 自定义(CUSTOM)：锥体
  // 扩展风格 — 用于动态注册的 Agent
  | 'hexPrism'       // 六棱柱 — 稳重坚固
  | 'doubleTorus'    // 双环体 — 互联循环
  | 'crystal'        // 水晶簇 — 锐利棱角
  | 'starTetra'      // 星形四面体 — 放射能量
  | 'spiralHelix'    // 螺旋体 — 持续演化
  | 'diamond'        // 钻石体 — 高贵闪耀
  | 'pyramid'        // 金字塔 — 古老智慧
  | 'mobius'         // 莫比乌斯环 — 无限循环
  | 'fragmentedCube' // 碎片方块 — 重构解构
  | 'lotus'          // 莲花体 — 层层绽放

/** 类别 -> 核心几何体类型 */
const CATEGORY_GEOMETRY: Record<AgentCategory, VisualStyleType> = {
  [AgentCategory.SEARCH]: 'octahedron',
  [AgentCategory.CODE]: 'icosahedron',
  [AgentCategory.ANALYSIS]: 'dodecahedron',
  [AgentCategory.CREATIVE]: 'torusKnot',
  [AgentCategory.GENERAL]: 'sphere',
  [AgentCategory.CUSTOM]: 'cone',
}

/** 扩展风格 -> 主色 */
const EXTENDED_COLORS: Record<string, string> = {
  hexPrism: '#20c997',
  doubleTorus: '#4dabf7',
  crystal: '#e599f7',
  starTetra: '#ffa94d',
  spiralHelix: '#69db7c',
  diamond: '#fcc419',
  pyramid: '#e8590c',
  mobius: '#748ffc',
  fragmentedCube: '#ff6b6b',
  lotus: '#f06595',
}

/** 扩展风格 -> 辅色 */
const EXTENDED_ACCENTS: Record<string, string> = {
  hexPrism: '#12b886',
  doubleTorus: '#339af0',
  crystal: '#cc5de8',
  starTetra: '#fd7e14',
  spiralHelix: '#37b24d',
  diamond: '#fab005',
  pyramid: '#d9480f',
  mobius: '#5c7cfa',
  fragmentedCube: '#e03131',
  lotus: '#d6336c',
}

/** 扩展风格 -> 旋转速度 */
const EXTENDED_ROTATION: Record<string, number> = {
  hexPrism: 0.25,
  doubleTorus: 0.35,
  crystal: 0.15,
  starTetra: 0.6,
  spiralHelix: 0.45,
  diamond: 0.2,
  pyramid: 0.18,
  mobius: 0.5,
  fragmentedCube: 0.3,
  lotus: 0.12,
}

/** 所有扩展风格列表（用于随机分配） */
const EXTENDED_STYLES: VisualStyleType[] = [
  'hexPrism', 'doubleTorus', 'crystal', 'starTetra', 'spiralHelix',
  'diamond', 'pyramid', 'mobius', 'fragmentedCube', 'lotus',
]

/** 基于Agent ID的确定性随机样式分配 — 同一Agent总是获得相同样式 */
const styleAssignmentCache = new Map<string, VisualStyleType>()

export function getVisualStyleForAgent(agentId: string, category: AgentCategory): VisualStyleType {
  // 内置类别直接返回对应样式
  if (category !== AgentCategory.CUSTOM) {
    return CATEGORY_GEOMETRY[category]
  }
  // 自定义类别：缓存 + 确定性随机
  if (styleAssignmentCache.has(agentId)) {
    return styleAssignmentCache.get(agentId)!
  }
  // 基于ID的简单哈希确定索引
  let hash = 0
  for (let i = 0; i < agentId.length; i++) {
    hash = ((hash << 5) - hash + agentId.charCodeAt(i)) | 0
  }
  const style = EXTENDED_STYLES[Math.abs(hash) % EXTENDED_STYLES.length]
  styleAssignmentCache.set(agentId, style)
  return style
}

/** 获取 Agent 的主色 */
export function getAgentColor(agentId: string, category: AgentCategory): string {
  if (category !== AgentCategory.CUSTOM) return CATEGORY_COLORS[category]
  const style = getVisualStyleForAgent(agentId, category)
  return EXTENDED_COLORS[style] || CATEGORY_COLORS[AgentCategory.CUSTOM]
}

/** 获取 Agent 的辅色 */
function getAgentAccent(agentId: string, category: AgentCategory): string {
  if (category !== AgentCategory.CUSTOM) return CATEGORY_ACCENT[category]
  const style = getVisualStyleForAgent(agentId, category)
  return EXTENDED_ACCENTS[style] || CATEGORY_ACCENT[AgentCategory.CUSTOM]
}

/** 获取旋转速度 */
function getAgentRotationSpeed(agentId: string, category: AgentCategory): number {
  if (category !== AgentCategory.CUSTOM) return CATEGORY_ROTATION_SPEED[category]
  const style = getVisualStyleForAgent(agentId, category)
  return EXTENDED_ROTATION[style] || 0.4
}

// ==================== 核心几何体组件 ====================

const CoreGeometry: React.FC<{
  type: VisualStyleType
}> = React.memo(({ type }) => {
  switch (type) {
    case 'octahedron':
      return <octahedronGeometry args={[0.45, 0]} />
    case 'icosahedron':
      return <icosahedronGeometry args={[0.45, 1]} />
    case 'dodecahedron':
      return <dodecahedronGeometry args={[0.45, 0]} />
    case 'torusKnot':
      return <torusKnotGeometry args={[0.3, 0.1, 64, 16, 2, 3]} />
    case 'cone':
      return <coneGeometry args={[0.4, 0.8, 6]} />
    case 'sphere':
      return <sphereGeometry args={[0.42, 32, 32]} />
    // ====== 扩展几何体 ======
    case 'hexPrism':
      // 六棱柱
      return <cylinderGeometry args={[0.38, 0.38, 0.65, 6]} />
    case 'doubleTorus':
      // 双环：使用大圆环
      return <torusGeometry args={[0.32, 0.1, 16, 48]} />
    case 'crystal':
      // 水晶簇：拉长的八面体
      return <octahedronGeometry args={[0.4, 2]} />
    case 'starTetra':
      // 星形四面体：用尖锥组合 → 使用高细分十二面体近似
      return <dodecahedronGeometry args={[0.42, 1]} />
    case 'spiralHelix':
      // 螺旋：环面纽结变种(3,2)
      return <torusKnotGeometry args={[0.28, 0.08, 80, 12, 3, 2]} />
    case 'diamond':
      // 钻石：高细分八面体上半大下半小
      return <octahedronGeometry args={[0.42, 3]} />
    case 'pyramid':
      // 金字塔：4面锥
      return <coneGeometry args={[0.42, 0.75, 4]} />
    case 'mobius':
      // 莫比乌斯：扁平环面纽结
      return <torusKnotGeometry args={[0.3, 0.06, 100, 8, 2, 3]} />
    case 'fragmentedCube':
      // 碎片方块：低细分球体近似
      return <icosahedronGeometry args={[0.4, 0]} />
    case 'lotus':
      // 莲花：环面纽结(5,3)
      return <torusKnotGeometry args={[0.25, 0.1, 80, 16, 5, 3]} />
    default:
      return <sphereGeometry args={[0.42, 32, 32]} />
  }
})
CoreGeometry.displayName = 'CoreGeometry'

// ==================== 轨道环组件 ====================

const OrbitRing: React.FC<{
  color: string
  radius: number
  speed: number
  tilt: [number, number, number]
}> = React.memo(({ color, radius, speed, tilt }) => {
  const ringRef = useRef<THREE.Mesh>(null!)

  useFrame((_, delta) => {
    if (ringRef.current) {
      ringRef.current.rotation.z += delta * speed
    }
  })

  return (
    <mesh ref={ringRef} rotation={tilt}>
      <torusGeometry args={[radius, 0.012, 8, 64]} />
      <meshBasicMaterial color={color} transparent opacity={0.5} />
    </mesh>
  )
})
OrbitRing.displayName = 'OrbitRing'

// ==================== 浮动碎片组件 ====================

const FloatingShard: React.FC<{
  color: string
  orbitRadius: number
  orbitSpeed: number
  orbitPhase: number
  size: number
}> = React.memo(({ color, orbitRadius, orbitSpeed, orbitPhase, size }) => {
  const meshRef = useRef<THREE.Mesh>(null!)
  const timeOffset = useRef(orbitPhase)

  useFrame((_, delta) => {
    timeOffset.current += delta * orbitSpeed
    if (meshRef.current) {
      const angle = timeOffset.current
      meshRef.current.position.x = orbitRadius * Math.cos(angle)
      meshRef.current.position.z = orbitRadius * Math.sin(angle)
      meshRef.current.position.y = 0.15 * Math.sin(angle * 2)
      meshRef.current.rotation.x += delta * 2
      meshRef.current.rotation.y += delta * 1.5
    }
  })

  return (
    <mesh ref={meshRef}>
      <octahedronGeometry args={[size, 0]} />
      <meshStandardMaterial
        color={color}
        emissive={color}
        emissiveIntensity={0.6}
        roughness={0.2}
        metalness={0.8}
      />
    </mesh>
  )
})
FloatingShard.displayName = 'FloatingShard'

// ==================== 扩展装饰：光环碎片（双环体用） ====================

const RingShard: React.FC<{
  color: string
  radius: number
  speed: number
  phase: number
  arcLength: number
}> = React.memo(({ color, radius, speed, phase, arcLength }) => {
  const groupRef = useRef<THREE.Group>(null!)
  const timeRef = useRef(phase)

  useFrame((_, delta) => {
    timeRef.current += delta * speed
    if (groupRef.current) {
      groupRef.current.rotation.y = timeRef.current
    }
  })

  const points = useMemo(() => {
    const pts: [number, number, number][] = []
    const segments = 12
    for (let i = 0; i <= segments; i++) {
      const angle = (i / segments) * arcLength
      pts.push([
        radius * Math.cos(angle),
        0,
        radius * Math.sin(angle),
      ])
    }
    return pts
  }, [radius, arcLength])

  return (
    <group ref={groupRef}>
      {/* 用小球近似弧段 */}
      {points.map((pt, i) => (
        <mesh key={i} position={pt}>
          <sphereGeometry args={[0.02, 6, 6]} />
          <meshBasicMaterial color={color} transparent opacity={0.6} />
        </mesh>
      ))}
    </group>
  )
})
RingShard.displayName = 'RingShard'

// ==================== 扩展装饰：脉冲波纹（用于金字塔/钻石等） ====================

const PulseRipple: React.FC<{
  color: string
  baseRadius: number
  speed: number
}> = React.memo(({ color, baseRadius, speed }) => {
  const ringRef = useRef<THREE.Mesh>(null!)
  const timeRef = useRef(0)
  const matRef = useRef<THREE.MeshBasicMaterial>(null!)

  useFrame((_, delta) => {
    timeRef.current += delta * speed
    if (ringRef.current && matRef.current) {
      const t = timeRef.current % 2 // 2秒一个周期
      const scale = 1 + t * 0.6
      ringRef.current.scale.set(scale, 1, scale)
      matRef.current.opacity = 0.25 * (1 - t / 2)
    }
  })

  return (
    <mesh ref={ringRef} rotation={[Math.PI / 2, 0, 0]} position={[0, -0.3, 0]}>
      <torusGeometry args={[baseRadius, 0.008, 8, 64]} />
      <meshBasicMaterial ref={matRef} color={color} transparent opacity={0.25} side={THREE.DoubleSide} />
    </mesh>
  )
})
PulseRipple.displayName = 'PulseRipple'

// ==================== 扩展装饰：旋转方块碎片 ====================

const CubeFragment: React.FC<{
  color: string
  orbitRadius: number
  speed: number
  phase: number
  size: number
}> = React.memo(({ color, orbitRadius, speed, phase, size }) => {
  const meshRef = useRef<THREE.Mesh>(null!)
  const timeRef = useRef(phase)

  useFrame((_, delta) => {
    timeRef.current += delta * speed
    if (meshRef.current) {
      const angle = timeRef.current
      meshRef.current.position.x = orbitRadius * Math.cos(angle)
      meshRef.current.position.z = orbitRadius * Math.sin(angle)
      meshRef.current.position.y = 0.2 * Math.sin(angle * 1.5)
      meshRef.current.rotation.x += delta * 1.5
      meshRef.current.rotation.z += delta * 1.2
    }
  })

  return (
    <mesh ref={meshRef}>
      <boxGeometry args={[size, size, size]} />
      <meshStandardMaterial
        color={color}
        emissive={color}
        emissiveIntensity={0.5}
        roughness={0.3}
        metalness={0.7}
      />
    </mesh>
  )
})
CubeFragment.displayName = 'CubeFragment'

// ==================== 扩展装饰：花瓣层（莲花体用） ====================

const PetalLayer: React.FC<{
  color: string
  petalCount: number
  radius: number
  tiltAngle: number
  rotSpeed: number
}> = React.memo(({ color, petalCount, radius, tiltAngle, rotSpeed }) => {
  const groupRef = useRef<THREE.Group>(null!)

  useFrame((_, delta) => {
    if (groupRef.current) {
      groupRef.current.rotation.y += delta * rotSpeed
    }
  })

  return (
    <group ref={groupRef}>
      {Array.from({ length: petalCount }, (_, i) => {
        const angle = (2 * Math.PI * i) / petalCount
        return (
          <mesh
            key={i}
            position={[
              radius * Math.cos(angle) * 0.7,
              0,
              radius * Math.sin(angle) * 0.7,
            ]}
            rotation={[tiltAngle, angle, 0]}
          >
            <sphereGeometry args={[0.15, 8, 8, 0, Math.PI, 0, Math.PI / 2]} />
            <meshPhysicalMaterial
              color={color}
              emissive={color}
              emissiveIntensity={0.3}
              roughness={0.2}
              metalness={0.5}
              clearcoat={0.6}
              transparent
              opacity={0.7}
              side={THREE.DoubleSide}
            />
          </mesh>
        )
      })}
    </group>
  )
})
PetalLayer.displayName = 'PetalLayer'

// ==================== 类别装饰配置 ====================

interface CategoryDecorConfig {
  orbitRings: Array<{ radius: number; speed: number; tilt: [number, number, number] }>
  shards: Array<{ orbitRadius: number; orbitSpeed: number; phase: number; size: number }>
  innerGlowScale: number
  outerGlowScale: number
}

const CATEGORY_DECOR: Record<AgentCategory, CategoryDecorConfig> = {
  [AgentCategory.SEARCH]: {
    orbitRings: [
      { radius: 0.7, speed: 1.2, tilt: [Math.PI / 3, 0, 0] },
      { radius: 0.85, speed: -0.8, tilt: [-Math.PI / 4, Math.PI / 6, 0] },
    ],
    shards: [
      { orbitRadius: 0.7, orbitSpeed: 1.2, phase: 0, size: 0.04 },
      { orbitRadius: 0.85, orbitSpeed: 0.8, phase: Math.PI, size: 0.035 },
    ],
    innerGlowScale: 1.4,
    outerGlowScale: 1.8,
  },
  [AgentCategory.CODE]: {
    orbitRings: [
      { radius: 0.75, speed: 0.6, tilt: [Math.PI / 2, 0, 0] },
    ],
    shards: [
      { orbitRadius: 0.75, orbitSpeed: 0.6, phase: 0, size: 0.05 },
      { orbitRadius: 0.75, orbitSpeed: 0.6, phase: Math.PI * 2 / 3, size: 0.04 },
      { orbitRadius: 0.75, orbitSpeed: 0.6, phase: Math.PI * 4 / 3, size: 0.04 },
    ],
    innerGlowScale: 1.35,
    outerGlowScale: 1.7,
  },
  [AgentCategory.ANALYSIS]: {
    orbitRings: [
      { radius: 0.65, speed: 0.5, tilt: [Math.PI / 2, 0, 0] },
      { radius: 0.8, speed: -0.3, tilt: [0, 0, Math.PI / 2] },
      { radius: 0.95, speed: 0.2, tilt: [Math.PI / 6, Math.PI / 4, 0] },
    ],
    shards: [
      { orbitRadius: 0.65, orbitSpeed: 0.5, phase: 0, size: 0.035 },
      { orbitRadius: 0.8, orbitSpeed: 0.3, phase: 1, size: 0.03 },
      { orbitRadius: 0.95, orbitSpeed: 0.2, phase: 2, size: 0.03 },
    ],
    innerGlowScale: 1.45,
    outerGlowScale: 1.85,
  },
  [AgentCategory.CREATIVE]: {
    orbitRings: [
      { radius: 0.8, speed: -1.5, tilt: [Math.PI / 5, Math.PI / 5, 0] },
    ],
    shards: [
      { orbitRadius: 0.6, orbitSpeed: -1.0, phase: 0, size: 0.04 },
      { orbitRadius: 0.8, orbitSpeed: -1.5, phase: Math.PI / 2, size: 0.035 },
      { orbitRadius: 0.55, orbitSpeed: 1.3, phase: Math.PI, size: 0.03 },
    ],
    innerGlowScale: 1.5,
    outerGlowScale: 1.9,
  },
  [AgentCategory.GENERAL]: {
    orbitRings: [
      { radius: 0.7, speed: 0.4, tilt: [Math.PI / 2, 0, 0] },
    ],
    shards: [
      { orbitRadius: 0.7, orbitSpeed: 0.4, phase: 0, size: 0.04 },
    ],
    innerGlowScale: 1.35,
    outerGlowScale: 1.7,
  },
  [AgentCategory.CUSTOM]: {
    orbitRings: [
      { radius: 0.75, speed: 0.9, tilt: [Math.PI / 2, 0, 0] },
      { radius: 0.9, speed: -0.5, tilt: [Math.PI / 3, 0, Math.PI / 4] },
    ],
    shards: [
      { orbitRadius: 0.75, orbitSpeed: 0.9, phase: 0, size: 0.04 },
      { orbitRadius: 0.9, orbitSpeed: 0.5, phase: Math.PI, size: 0.035 },
    ],
    innerGlowScale: 1.4,
    outerGlowScale: 1.75,
  },
}

/** 扩展风格的装饰配置 */
const EXTENDED_DECOR: Record<string, CategoryDecorConfig> = {
  hexPrism: {
    orbitRings: [
      { radius: 0.7, speed: 0.5, tilt: [Math.PI / 2, 0, 0] },
      { radius: 0.7, speed: -0.5, tilt: [Math.PI / 6, Math.PI / 3, 0] },
    ],
    shards: [
      { orbitRadius: 0.7, orbitSpeed: 0.5, phase: 0, size: 0.035 },
      { orbitRadius: 0.7, orbitSpeed: 0.5, phase: Math.PI * 2 / 3, size: 0.03 },
      { orbitRadius: 0.7, orbitSpeed: 0.5, phase: Math.PI * 4 / 3, size: 0.03 },
    ],
    innerGlowScale: 1.4,
    outerGlowScale: 1.75,
  },
  doubleTorus: {
    orbitRings: [
      { radius: 0.75, speed: 0.7, tilt: [Math.PI / 2, 0, 0] },
      { radius: 0.75, speed: -0.7, tilt: [0, 0, Math.PI / 2] },
    ],
    shards: [
      { orbitRadius: 0.75, orbitSpeed: 0.7, phase: 0, size: 0.03 },
      { orbitRadius: 0.75, orbitSpeed: -0.7, phase: Math.PI / 2, size: 0.03 },
    ],
    innerGlowScale: 1.4,
    outerGlowScale: 1.8,
  },
  crystal: {
    orbitRings: [
      { radius: 0.85, speed: 0.2, tilt: [Math.PI / 3, Math.PI / 6, 0] },
    ],
    shards: [
      { orbitRadius: 0.85, orbitSpeed: 0.2, phase: 0, size: 0.04 },
      { orbitRadius: 0.6, orbitSpeed: 0.4, phase: 1, size: 0.035 },
      { orbitRadius: 0.6, orbitSpeed: 0.4, phase: 3, size: 0.03 },
    ],
    innerGlowScale: 1.5,
    outerGlowScale: 1.9,
  },
  starTetra: {
    orbitRings: [
      { radius: 0.65, speed: 1.0, tilt: [Math.PI / 2, 0, 0] },
      { radius: 0.9, speed: -0.6, tilt: [Math.PI / 4, Math.PI / 5, 0] },
    ],
    shards: [
      { orbitRadius: 0.65, orbitSpeed: 1.0, phase: 0, size: 0.045 },
      { orbitRadius: 0.9, orbitSpeed: 0.6, phase: Math.PI, size: 0.035 },
      { orbitRadius: 0.5, orbitSpeed: -1.5, phase: 1, size: 0.03 },
    ],
    innerGlowScale: 1.45,
    outerGlowScale: 1.85,
  },
  spiralHelix: {
    orbitRings: [
      { radius: 0.7, speed: 0.8, tilt: [Math.PI / 2, 0, 0] },
    ],
    shards: [
      { orbitRadius: 0.7, orbitSpeed: 0.8, phase: 0, size: 0.035 },
      { orbitRadius: 0.7, orbitSpeed: 0.8, phase: Math.PI, size: 0.035 },
    ],
    innerGlowScale: 1.35,
    outerGlowScale: 1.75,
  },
  diamond: {
    orbitRings: [
      { radius: 0.8, speed: 0.3, tilt: [Math.PI / 2, 0, 0] },
    ],
    shards: [
      { orbitRadius: 0.8, orbitSpeed: 0.3, phase: 0, size: 0.03 },
      { orbitRadius: 0.55, orbitSpeed: 0.5, phase: 0.5, size: 0.025 },
    ],
    innerGlowScale: 1.5,
    outerGlowScale: 1.9,
  },
  pyramid: {
    orbitRings: [
      { radius: 0.7, speed: 0.35, tilt: [Math.PI / 2, 0, 0] },
    ],
    shards: [
      { orbitRadius: 0.7, orbitSpeed: 0.35, phase: 0, size: 0.035 },
      { orbitRadius: 0.7, orbitSpeed: 0.35, phase: Math.PI, size: 0.035 },
    ],
    innerGlowScale: 1.35,
    outerGlowScale: 1.7,
  },
  mobius: {
    orbitRings: [
      { radius: 0.8, speed: 0.6, tilt: [Math.PI / 3, 0, 0] },
      { radius: 0.8, speed: -0.4, tilt: [-Math.PI / 4, Math.PI / 5, 0] },
    ],
    shards: [
      { orbitRadius: 0.8, orbitSpeed: 0.6, phase: 0, size: 0.03 },
      { orbitRadius: 0.8, orbitSpeed: -0.4, phase: 2, size: 0.03 },
    ],
    innerGlowScale: 1.4,
    outerGlowScale: 1.8,
  },
  fragmentedCube: {
    orbitRings: [
      { radius: 0.75, speed: 0.5, tilt: [Math.PI / 2, 0, 0] },
    ],
    shards: [],
    innerGlowScale: 1.35,
    outerGlowScale: 1.75,
  },
  lotus: {
    orbitRings: [
      { radius: 0.6, speed: 0.2, tilt: [Math.PI / 2, 0, 0] },
    ],
    shards: [
      { orbitRadius: 0.9, orbitSpeed: 0.3, phase: 0, size: 0.025 },
      { orbitRadius: 0.9, orbitSpeed: 0.3, phase: Math.PI, size: 0.025 },
    ],
    innerGlowScale: 1.45,
    outerGlowScale: 1.85,
  },
}

/** 获取 Agent 的装饰配置 */
function getAgentDecor(agentId: string, category: AgentCategory): CategoryDecorConfig {
  if (category !== AgentCategory.CUSTOM) return CATEGORY_DECOR[category]
  const style = getVisualStyleForAgent(agentId, category)
  return EXTENDED_DECOR[style] || CATEGORY_DECOR[AgentCategory.CUSTOM]
}

// ==================== Props ====================

interface AgentNode3DProps {
  data: AgentVisualData
  isSelected: boolean
  onSelect: (id: string) => void
}

// ==================== AgentNode3D ====================

const AgentNode3D: React.FC<AgentNode3DProps> = React.memo(({ data, isSelected, onSelect }) => {
  const groupRef = useRef<THREE.Group>(null!)
  const meshRef = useRef<THREE.Mesh>(null!)
  const innerGlowRef = useRef<THREE.Mesh>(null!)
  const outerGlowRef = useRef<THREE.Mesh>(null!)
  const selectionRingRef = useRef<THREE.Group>(null!)
  const hoverScaleRef = useRef(1)
  const timeRef = useRef(Math.random() * Math.PI * 2)

  const category = data.category
  const statusColor = STATUS_COLORS[data.status]
  const categoryColor = getAgentColor(data.id, category)
  const categoryAccent = getAgentAccent(data.id, category)
  const baseColor = data.status === 'idle' ? categoryColor : statusColor
  const emissiveIntensity = STATUS_EMISSIVE_INTENSITY[data.status]
  const geoType = getVisualStyleForAgent(data.id, category)
  const decor = getAgentDecor(data.id, category)
  const rotSpeed = getAgentRotationSpeed(data.id, category)

  // 预计算材质颜色
  const baseColorObj = useMemo(() => new THREE.Color(baseColor), [baseColor])
  const emissiveColorObj = useMemo(() => new THREE.Color(baseColor), [baseColor])

  // 内层光晕材质（类别色）
  const innerGlowMat = useMemo(
    () => new THREE.MeshBasicMaterial({
      color: categoryAccent,
      transparent: true,
      opacity: 0.08,
      side: THREE.BackSide,
    }),
    [categoryAccent]
  )

  // 外层光晕材质（状态色）
  const outerGlowMat = useMemo(
    () => new THREE.MeshBasicMaterial({
      color: baseColor,
      transparent: true,
      opacity: 0.05,
      side: THREE.BackSide,
    }),
    [baseColor]
  )

  // 选中环材质
  const selectionRingMat = useMemo(
    () => new THREE.MeshBasicMaterial({
      color: '#ffffff',
      transparent: true,
      opacity: 0.7,
      side: THREE.DoubleSide,
    }),
    []
  )

  // 动画帧
  useFrame((_, delta) => {
    if (!groupRef.current || !meshRef.current) return

    timeRef.current += delta
    const t = timeRef.current
    let scale = 1

    // 主体旋转（类别/风格特定速度）
    meshRef.current.rotation.y += delta * rotSpeed
    meshRef.current.rotation.x += delta * rotSpeed * 0.3

    // 状态动画
    switch (data.status) {
      case 'running': {
        const pulse = 1 + 0.06 * Math.sin(t * 3)
        scale = pulse
        if (innerGlowRef.current) {
          innerGlowRef.current.scale.setScalar(decor.innerGlowScale * pulse)
          innerGlowMat.opacity = 0.12 + 0.08 * Math.sin(t * 3)
        }
        if (outerGlowRef.current) {
          outerGlowRef.current.scale.setScalar(decor.outerGlowScale * pulse)
          outerGlowMat.opacity = 0.08 + 0.06 * Math.sin(t * 3)
        }
        break
      }
      case 'idle': {
        const breath = 1 + 0.012 * Math.sin(t * 1.2)
        scale = breath
        if (innerGlowRef.current) {
          innerGlowRef.current.scale.setScalar(decor.innerGlowScale)
          innerGlowMat.opacity = 0.06 + 0.02 * Math.sin(t * 1.2)
        }
        if (outerGlowRef.current) {
          outerGlowRef.current.scale.setScalar(decor.outerGlowScale)
          outerGlowMat.opacity = 0.03 + 0.015 * Math.sin(t * 1.2)
        }
        break
      }
      case 'completed': {
        scale = 1 + 0.015 * Math.sin(t * 2)
        if (innerGlowRef.current) {
          innerGlowRef.current.scale.setScalar(decor.innerGlowScale)
          innerGlowMat.opacity = 0.1
        }
        if (outerGlowRef.current) {
          outerGlowRef.current.scale.setScalar(decor.outerGlowScale)
          outerGlowMat.opacity = 0.06
        }
        break
      }
      case 'failed': {
        const blink = Math.sin(t * 6) > 0 ? 1 : 0.5
        scale = 1
        if (innerGlowRef.current) {
          innerGlowRef.current.scale.setScalar(decor.innerGlowScale * 1.1)
          innerGlowMat.opacity = 0.18 * blink
        }
        if (outerGlowRef.current) {
          outerGlowRef.current.scale.setScalar(decor.outerGlowScale * 1.1)
          outerGlowMat.opacity = 0.12 * blink
        }
        break
      }
      case 'pending': {
        scale = 0.95
        if (innerGlowRef.current) {
          innerGlowRef.current.scale.setScalar(decor.innerGlowScale * 0.9)
          innerGlowMat.opacity = 0.03
        }
        if (outerGlowRef.current) {
          outerGlowRef.current.scale.setScalar(decor.outerGlowScale * 0.9)
          outerGlowMat.opacity = 0.015
        }
        break
      }
      case 'waiting_input': {
        const jitter = 1 + 0.015 * Math.sin(t * 8)
        scale = jitter
        if (innerGlowRef.current) {
          innerGlowRef.current.scale.setScalar(decor.innerGlowScale)
          innerGlowMat.opacity = 0.1 + 0.05 * Math.sin(t * 4)
        }
        if (outerGlowRef.current) {
          outerGlowRef.current.scale.setScalar(decor.outerGlowScale)
          outerGlowMat.opacity = 0.06 + 0.03 * Math.sin(t * 4)
        }
        break
      }
      case 'timeout': {
        const fade = 0.7 + 0.3 * Math.sin(t * 2)
        scale = 1
        if (innerGlowRef.current) {
          innerGlowRef.current.scale.setScalar(decor.innerGlowScale)
          innerGlowMat.opacity = 0.08 * fade
        }
        if (outerGlowRef.current) {
          outerGlowRef.current.scale.setScalar(decor.outerGlowScale)
          outerGlowMat.opacity = 0.05 * fade
        }
        break
      }
    }

    // hover 缩放平滑插值
    const targetScale = scale * hoverScaleRef.current
    const currentScale = groupRef.current.scale.x
    const newScale = THREE.MathUtils.lerp(currentScale, targetScale, 0.1)
    groupRef.current.scale.setScalar(newScale)

    // 选中环旋转
    if (selectionRingRef.current && isSelected) {
      selectionRingRef.current.rotation.z += delta * 1.2
      selectionRingRef.current.rotation.y += delta * 0.5
    }
  })

  // 鼠标交互
  const onPointerOver = useMemo(
    () => () => {
      hoverScaleRef.current = 1.15
      document.body.style.cursor = 'pointer'
    },
    []
  )
  const onPointerOut = useMemo(
    () => () => {
      hoverScaleRef.current = 1
      document.body.style.cursor = 'default'
    },
    []
  )
  const onClick = useMemo(() => () => onSelect(data.id), [data.id, onSelect])

  // 是否为扩展风格（需额外装饰组件）
  const isExtendedStyle = category === AgentCategory.CUSTOM && EXTENDED_DECOR[geoType] !== undefined

  return (
    <group ref={groupRef} position={data.position}>
      {/* 核心几何体 */}
      <mesh
        ref={meshRef}
        onPointerOver={onPointerOver}
        onPointerOut={onPointerOut}
        onClick={onClick}
      >
        <CoreGeometry type={geoType} />
        <meshPhysicalMaterial
          color={baseColorObj}
          emissive={emissiveColorObj}
          emissiveIntensity={emissiveIntensity}
          roughness={0.15}
          metalness={0.7}
          clearcoat={0.4}
          clearcoatRoughness={0.2}
          transparent={data.status === 'pending'}
          opacity={data.status === 'pending' ? 0.5 : 1}
          envMapIntensity={1.2}
        />
      </mesh>

      {/* 内层光晕（类别色） */}
      <mesh ref={innerGlowRef} material={innerGlowMat}>
        <sphereGeometry args={[0.6, 24, 24]} />
      </mesh>

      {/* 外层光晕（状态色） */}
      <mesh ref={outerGlowRef} material={outerGlowMat}>
        <sphereGeometry args={[0.75, 20, 20]} />
      </mesh>

      {/* 通用装饰 - 轨道环 */}
      {decor.orbitRings.map((ring, i) => (
        <OrbitRing
          key={`ring-${i}`}
          color={categoryAccent}
          radius={ring.radius}
          speed={ring.speed}
          tilt={ring.tilt}
        />
      ))}

      {/* 通用装饰 - 浮动碎片 */}
      {decor.shards.map((shard, i) => (
        <FloatingShard
          key={`shard-${i}`}
          color={categoryAccent}
          orbitRadius={shard.orbitRadius}
          orbitSpeed={shard.orbitSpeed}
          orbitPhase={shard.phase}
          size={shard.size}
        />
      ))}

      {/* ====== 扩展风格专属装饰 ====== */}

      {/* 脉冲波纹 — 金字塔/钻石 */}
      {(geoType === 'pyramid' || geoType === 'diamond') && (
        <PulseRipple color={categoryAccent} baseRadius={0.8} speed={0.8} />
      )}

      {/* 方块碎片 — 碎片方块 */}
      {geoType === 'fragmentedCube' && (
        <>
          <CubeFragment color={categoryAccent} orbitRadius={0.7} speed={0.5} phase={0} size={0.06} />
          <CubeFragment color={categoryAccent} orbitRadius={0.7} speed={0.5} phase={Math.PI / 2} size={0.05} />
          <CubeFragment color={categoryAccent} orbitRadius={0.7} speed={0.5} phase={Math.PI} size={0.06} />
          <CubeFragment color={categoryAccent} orbitRadius={0.7} speed={0.5} phase={Math.PI * 1.5} size={0.05} />
        </>
      )}

      {/* 光环碎片 — 双环体 */}
      {geoType === 'doubleTorus' && (
        <>
          <RingShard color={categoryAccent} radius={0.9} speed={0.5} phase={0} arcLength={Math.PI / 3} />
          <RingShard color={categoryAccent} radius={0.9} speed={-0.4} phase={Math.PI} arcLength={Math.PI / 4} />
        </>
      )}

      {/* 花瓣层 — 莲花体 */}
      {geoType === 'lotus' && (
        <>
          <PetalLayer color={categoryColor} petalCount={5} radius={0.55} tiltAngle={0.6} rotSpeed={0.15} />
          <PetalLayer color={categoryAccent} petalCount={7} radius={0.8} tiltAngle={0.4} rotSpeed={-0.1} />
        </>
      )}

      {/* 选中指示 - 双环 */}
      {isSelected && (
        <group ref={selectionRingRef}>
          <mesh material={selectionRingMat} rotation={[Math.PI / 2, 0, 0]}>
            <torusGeometry args={[0.9, 0.015, 8, 64]} />
          </mesh>
          <mesh material={selectionRingMat} rotation={[Math.PI / 3, Math.PI / 4, 0]}>
            <torusGeometry args={[0.95, 0.01, 8, 64]} />
          </mesh>
        </group>
      )}

      {/* 活跃任务指示器 - 脉冲点 */}
      {data.activeTaskCount > 0 && (
        <mesh position={[0.4, 0.45, 0.35]}>
          <sphereGeometry args={[0.06, 12, 12]} />
          <meshStandardMaterial
            color="#51cf66"
            emissive="#51cf66"
            emissiveIntensity={1.2}
          />
        </mesh>
      )}

      {/* 协作指示器 - 顶部钻石 */}
      {data.isCollaborating && (
        <mesh position={[0, 0.7, 0]} rotation={[0, Math.PI / 4, 0]}>
          <octahedronGeometry args={[0.06, 0]} />
          <meshStandardMaterial
            color="#ffd43b"
            emissive="#ffd43b"
            emissiveIntensity={0.9}
          />
        </mesh>
      )}

      {/* 名称标签 - 始终朝向相机 */}
      <Billboard position={[0, -1.05, 0]}>
        <Text
          fontSize={0.16}
          color="#e0e0e0"
          anchorX="center"
          anchorY="middle"
          outlineWidth={0.012}
          outlineColor="#050510"
          fontWeight={500}
        >
          {data.icon} {data.name}
        </Text>
      </Billboard>

      {/* 状态标签 */}
      {(data.status === 'running' || data.status === 'failed' || data.status === 'waiting_input') && (
        <Billboard position={[0, 1.05, 0]}>
          <Text
            fontSize={0.1}
            color={statusColor}
            anchorX="center"
            anchorY="middle"
            outlineWidth={0.008}
            outlineColor="#050510"
          >
            {data.status === 'running' ? '● 运行中' : data.status === 'failed' ? '✕ 失败' : '⏳ 等待输入'}
          </Text>
        </Billboard>
      )}
    </group>
  )
})

AgentNode3D.displayName = 'AgentNode3D'

export default AgentNode3D
