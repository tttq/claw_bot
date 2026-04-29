---
name: wechat
app_name: 微信
aliases:
  - WeChat
  - wechat
  - weixin
description: 微信桌面客户端 — 支持发送消息、搜索联系人、文件传输等操作
shortcuts:
  搜索: Ctrl+F
  发送消息: Enter
  换行: Shift+Enter
  截图: Alt+A
  文件传输: Ctrl+Shift+F
operations:
  send_message:
    description: 发送消息给联系人
    steps:
      - action: 确认微信已打开
        target: 微信窗口/任务栏图标
        wait_after_ms: 2000
        note: 若未打开则使用open_app打开微信
      - action: 检查登录状态
        target: 登录二维码/主界面
        wait_after_ms: 1000
        note: 若显示二维码则提示用户扫码
      - action: 打开搜索框
        target: 搜索框
        shortcut: Ctrl+F
        wait_after_ms: 500
      - action: 输入联系人名称
        target: 搜索输入框
        input_placeholder: contact_name
        wait_after_ms: 800
        note: 输入后等待搜索结果
      - action: 点击搜索结果中的联系人
        target: 搜索结果列表
        wait_after_ms: 500
        note: 点击第一个匹配结果
      - action: 点击聊天输入框
        target: 消息输入框
        wait_after_ms: 300
      - action: 输入消息内容
        target: 消息输入框
        input_placeholder: message
        wait_after_ms: 300
      - action: 发送消息
        shortcut: Enter
        wait_after_ms: 500
        note: 确认消息已发送
  open_chat:
    description: 打开与某人的聊天窗口
    steps:
      - action: 确认微信已打开并登录
        wait_after_ms: 2000
      - action: 打开搜索
        shortcut: Ctrl+F
        wait_after_ms: 500
      - action: 输入联系人名称
        target: 搜索框
        input_placeholder: contact_name
        wait_after_ms: 800
      - action: 点击联系人
        target: 搜索结果
        wait_after_ms: 500
ui_hints:
  - element: 搜索框
    description: 微信主界面顶部的搜索区域
    typical_position: 窗口顶部居中
    look_for: 搜索图标或"搜索"文字
  - element: 聊天输入框
    description: 聊天窗口底部的文字输入区域
    typical_position: 窗口底部
    look_for: 空白输入区域或"按Enter发送"提示
  - element: 联系人列表
    description: 左侧的聊天/联系人列表
    typical_position: 窗口左侧
    look_for: 头像和名称列表
error_states:
  - name: 未登录
    detection: 显示二维码登录界面
    recovery: 提示用户需要扫码登录，使用fail结束任务
  - name: 联系人不存在
    detection: 搜索结果为空或显示"无搜索结果"
    recovery: 告知用户找不到该联系人，使用fail结束
  - name: 消息发送失败
    detection: 消息旁显示红色感叹号
    recovery: 等待几秒后重试发送
---

# 微信自动化技能

本技能定义了微信桌面客户端的自动化操作知识，CUA Agent在执行微信相关任务时会自动加载这些知识。

## 支持的操作

- **send_message**: 发送消息给指定联系人
- **open_chat**: 打开与某人的聊天窗口

## 使用示例

- "给张三发微信消息：你在干什么？"
- "在微信上找一下李四"
- "用微信给王五发文件"
