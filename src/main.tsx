// Claw Desktop - React 应用入口
// 职责：挂载根组件到 DOM，初始化 i18n 国际化和全局样式
import React from 'react'
import ReactDOM from 'react-dom/client'
import './i18n/config'   // 初始化 i18next 国际化（副作用导入，无需显式引用）
import App from './App.tsx'
import './index.css'     // 全局样式（Tailwind CSS 指令 + 自定义样式）

// 创建 React 根节点并渲染应用，启用 StrictMode 进行开发期额外检查
ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
