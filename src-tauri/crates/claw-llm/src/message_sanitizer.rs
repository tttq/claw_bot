// Claw Desktop - 消息清理器 - 清理发送给LLM的消息序列
// 功能：过滤无效角色、修复孤立工具结果、去重工具调用

use serde_json::Value;
use std::collections::HashSet;

/// 合法的消息角色列表
const VALID_ROLES: &[&str] = &["system", "user", "assistant", "tool", "function"];

/// 消息清理入口函数 - 执行所有清理步骤
pub fn sanitize_messages(messages: &mut Vec<Value>) {
    filter_invalid_roles(messages);
    fix_orphaned_tool_results(messages);
}

/// 过滤掉角色名无效的消息（不在合法角色列表中的消息会被丢弃）
fn filter_invalid_roles(messages: &mut Vec<Value>) {
    let mut valid_messages: Vec<Value> = Vec::new();
    
    for msg in messages.drain(..) {
        if let Some(role) = msg.get("role").and_then(|v| v.as_str()) {
            if VALID_ROLES.contains(&role) {
                valid_messages.push(msg);
            } else {
                log::debug!("[Sanitizer] Dropping message with invalid role: {}", role);
            }
        }
    }
    
    *messages = valid_messages;
}

/// 修复孤立的工具结果消息
/// 1. 删除没有对应工具调用的工具结果消息
/// 2. 为缺少结果消息的工具调用补充占位结果
fn fix_orphaned_tool_results(messages: &mut Vec<Value>) {
    // 收集所有助手消息中的工具调用ID
    let mut surviving_call_ids: HashSet<String> = HashSet::new();
    
    for msg in messages.iter() {
        if msg.get("role").and_then(|v| v.as_str()) == Some("assistant") {
            if let Some(tool_calls) = msg.get("tool_calls").and_then(|v| v.as_array()) {
                for tc in tool_calls {
                    if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                        if !id.is_empty() {
                            surviving_call_ids.insert(id.to_string());
                        }
                    }
                }
            }
        }
    }

    // 识别孤立的工具结果（没有对应工具调用的结果消息）
    let mut result_call_ids: HashSet<String> = HashSet::new();
    let mut orphaned_indices: Vec<usize> = Vec::new();
    
    for (idx, msg) in messages.iter().enumerate() {
        if msg.get("role").and_then(|v| v.as_str()) == Some("tool") {
            if let Some(call_id) = msg.get("tool_call_id").and_then(|v| v.as_str()) {
                result_call_ids.insert(call_id.to_string());
                if !surviving_call_ids.contains(call_id) {
                    orphaned_indices.push(idx);
                }
            }
        }
    }

    // 移除孤立的工具结果消息
    if !orphaned_indices.is_empty() {
        log::debug!("[Sanitizer] Removing {} orphaned tool results", orphaned_indices.len());
        for idx in orphaned_indices.into_iter().rev() {
            messages.remove(idx);
        }
    }

    // 找出缺少结果消息的工具调用ID
    let missing_results: Vec<String> = surviving_call_ids
        .difference(&result_call_ids)
        .cloned()
        .collect();

    // 为缺失的工具调用补充占位结果消息
    if !missing_results.is_empty() {
        log::debug!("[Sanitizer] Adding {} stub tool results", missing_results.len());
        let missing_set: HashSet<&str> = missing_results.iter().map(|s| s.as_str()).collect();
        
        let mut new_messages: Vec<Value> = Vec::new();
        for msg in messages.iter() {
            new_messages.push(msg.clone());
            
            if msg.get("role").and_then(|v| v.as_str()) == Some("assistant") {
                if let Some(tool_calls) = msg.get("tool_calls").and_then(|v| v.as_array()) {
                    for tc in tool_calls {
                        if let Some(call_id) = tc.get("id").and_then(|v| v.as_str()) {
                            if missing_set.contains(call_id) {
                                new_messages.push(serde_json::json!({
                                    "role": "tool",
                                    "content": "[Result unavailable]",
                                    "tool_call_id": call_id
                                }));
                            }
                        }
                    }
                }
            }
        }
        
        *messages = new_messages;
    }
}

/// 去重工具调用列表 - 移除名称和参数完全相同的重复工具调用
pub fn deduplicate_tool_calls(tool_calls: &[Value]) -> Vec<Value> {
    let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    let mut unique: Vec<Value> = Vec::new();

    for tc in tool_calls {
        let name = tc
            .get("function")
            .and_then(|f| f.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        
        let args = tc
            .get("function")
            .and_then(|f| f.get("arguments"))
            .and_then(|a| a.as_str())
            .unwrap_or("")
            .to_string();

        let key = (name.clone(), args);
        if seen.insert(key) {
            unique.push(tc.clone());
        } else {
            log::warn!("[Sanitizer] Removed duplicate tool call: {}", name);
        }
    }

    unique
}
