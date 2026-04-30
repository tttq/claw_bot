// Claw Desktop - 注册表 - 通用注册表实现
// 定义所有可用工具的元数据（名称、描述、输入 Schema）
// 提供 list_tools 命令供前端和 LLM 查询可用工具列表
// LLM 通过 tools 参数获取此列表以实现 function calling

use claw_types::common::ToolDefinition;
use serde_json::json;

/// 获取所有已注册工具的完整定义列表
/// 返回格式兼容 Anthropic tools API 和 OpenAI function calling
pub fn get_all_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        // ===== 文件操作类 =====
        ToolDefinition {
            name: "Read".to_string(),
            description: "Read the contents of a file. Use for viewing source code, configs, logs, or any text file.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Absolute or relative path to the file to read"},
                    "offset": {"type": "integer", "description": "Line number to start reading from (1-based, default: 1)"},
                    "limit": {"type": "integer", "description": "Maximum number of lines to read (default: all)"}
                },
                "required": ["file_path"]
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "Edit".to_string(),
            description: "Make edits to a file using string replacement. Finds old_string and replaces with new_string. Supports multiple edits in one call.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Path to the file to edit"},
                    "edits": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "old_string": {"type": "string", "description": "Exact text to find and replace"},
                                "new_string": {"type": "string", "description": "Replacement text"}
                            },
                            "required": ["old_string", "new_string"]
                        },
                        "description": "Array of edit operations"
                    },
                    "dry_run": {"type": "boolean", "description": "If true, only preview changes without writing"}
                },
                "required": ["file_path", "edits"]
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "Write".to_string(),
            description: "Write content to a file, creating it if it doesn't exist or overwriting if it does.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Path to write to"},
                    "content": {"type": "string", "description": "Full file content to write"},
                    "create_dirs": {"type": "boolean", "description": "Create parent directories if needed (default: true)"}
                },
                "required": ["file_path", "content"]
            }),
            category: None,
            tags: Vec::new(),
        },

        // ===== Shell 命令类 =====
        ToolDefinition {
            name: "Bash".to_string(),
            description: "Execute a shell command in the user's environment. Can run any CLI command, build tools, git operations, package managers, etc.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Shell command to execute"},
                    "working_dir": {"type": "string", "description": "Working directory for command execution"},
                    "timeout_secs": {"type": "integer", "description": "Timeout in seconds (default: 120)"}
                },
                "required": ["command"]
            }),
            category: None,
            tags: Vec::new(),
        },

        // ===== 文件搜索类 =====
        ToolDefinition {
            name: "Glob".to_string(),
            description: "Find files matching a glob pattern (e.g., '**/*.rs', 'src/**/*.tsx'). Supports ** wildcards.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Glob pattern to match files against"},
                    "path": {"type": "string", "description": "Directory to search in (default: current working directory)"},
                    "exclude_patterns": {"type": "array", "items": {"type": "string"}, "description": "Patterns to exclude (e.g., ['node_modules', '.git'])"}
                },
                "required": ["pattern"]
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "Grep".to_string(),
            description: "Search file contents using regex patterns. Like grep -rn but with structured output.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Regex pattern to search for"},
                    "path": {"type": "string", "description": "Directory or file path to search in"},
                    "include_pattern": {"type": "string", "description": "File glob filter (e.g., '*.rs')"},
                    "exclude_pattern": {"type": "string", "description": "File glob exclusion pattern"}
                },
                "required": ["pattern"]
            }),
            category: None,
            tags: Vec::new(),
        },

        // ===== 网络工具类 =====
        ToolDefinition {
            name: "WebFetch".to_string(),
            description: "Fetch and return the content of a specific URL. Use ONLY when the user explicitly asks to read a webpage or when you need to access a specific URL they provided. Do NOT use proactively for general questions.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {"type": "string", "description": "URL to fetch"},
                    "max_length": {"type": "integer", "description": "Max characters to return (default: 50000)"}
                },
                "required": ["url"]
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "WebSearch".to_string(),
            description: "Search the internet for real-time or current information. ONLY use when the user explicitly asks to search the web, or when you need up-to-date information you cannot provide from training data (e.g., current events, latest versions, recent news). Do NOT use for general knowledge, coding help, or questions you can answer directly.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Search query"},
                    "num_results": {"type": "integer", "description": "Number of results (default: 5)"}
                },
                "required": ["query"]
            }),
            category: None,
            tags: Vec::new(),
        },

        // ===== Agent 编排类 =====
        ToolDefinition {
            name: "Agent".to_string(),
            description: "Spawn a sub-agent to handle a subtask independently. The agent runs in its own context and returns a summary. Useful for parallelizing complex tasks.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "prompt": {"type": "string", "description": "Task description/prompt for the sub-agent"},
                    "mode": {"type": "string", "enum": ["fork", "background"], "description": "Execution mode (default: fork)"},
                    "model_override": {"type": "string", "description": "Override model for this agent invocation"},
                    "agent_id": {"type": "string", "description": "Optional agent ID to use its system prompt and model configuration"}
                },
                "required": ["prompt"]
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "TodoWrite".to_string(),
            description: "Create or update a todo list to track progress on multi-step tasks. Shows status icons [OK]/[..]/[  ].".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "todos": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "content": {"type": "string"},
                                "status": {"type": "string", "enum": ["pending", "in_progress", "completed"]},
                                "priority": {"type": "string", "enum": ["high", "medium", "low"]}
                            },
                            "required": ["content", "status"]
                        }
                    }
                },
                "required": ["todos"]
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "TaskCreate".to_string(),
            description: "Create a new background task that runs independently.".to_string(),
            input_schema: json!({"type":"object","properties":{"prompt":{"type":"string"},"description":{"type":"string"}},"required":["prompt"]}),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "TaskList".to_string(),
            description: "List all background tasks with their status.".to_string(),
            input_schema: json!({"type":"object","properties":{"status_filter":{"type":"string"}}}),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "Workflow".to_string(),
            description: "Execute a predefined workflow or task template with multiple steps.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Name of workflow"},
                    "steps": {"type": "array", "items": {"type": "object", "properties": {"action": {"type": "string"}, "params": {"type": "object"}}, "required": ["action"]}},
                    "inputs": {"type": "object", "description": "Workflow inputs"}
                },
                "required": ["name"]
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "Skill".to_string(),
            description: "Invoke a named skill (a reusable prompt template with access to specific tools). Skills encapsulate multi-step workflows like code review, testing, deployment, etc. Use when the user's task matches a known skill's purpose.".to_string(),
            input_schema: json!({"type":"object","properties":{"skill_name":{"type":"string","description":"Name of the skill to invoke"},"args":{"type":"object","description":"Arguments to pass to the skill"}},"required":["skill_name"]}),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "EnterPlanMode".to_string(),
            description: "Enter plan mode where the AI creates a detailed plan before making changes. Blocks actual modifications.".to_string(),
            input_schema: json!({"type":"object","properties":{}}),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "ExitPlanMode".to_string(),
            description: "Exit plan mode and begin executing the planned changes.".to_string(),
            input_schema: json!({"type":"object","properties":{}}),
            category: None,
            tags: Vec::new(),
        },

        // ===== 杂项辅助工具 =====
        ToolDefinition {
            name: "Brief".to_string(),
            description: "Send a brief message or notification to the user.".to_string(),
            input_schema: json!({"type":"object","properties":{"message":{"type":"string"},"attachments":{"type":"array","items":{"type":"string"}}},"required":["message"]}),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "Config".to_string(),
            description: "Read or update global application configuration settings.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "action": {"type": "string", "enum": ["get", "set", "list"], "description": "Configuration action"},
                    "key": {"type": "string", "description": "Configuration key (dot-notation supported)"},
                    "value": {"description": "Value to set (any type)"}
                },
                "required": ["action"]
            }),
            category: None,
            tags: Vec::new(),
        },

        // ===== Git 版本控制工具 =====
        ToolDefinition {
            name: "GitStatus".to_string(),
            description: "Show the working tree status - modified, added, deleted, untracked files.".to_string(),
            input_schema: json!({"type":"object","properties":{"working_dir":{"type":"string","description":"Repository path (default: current dir)"}}}),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "GitDiff".to_string(),
            description: "Show changes between commits, commit and working tree, etc. Supports per-file diff.".to_string(),
            input_schema: json!({"type":"object","properties":{"working_dir":{"type":"string"},"file_path":{"type":"string"},"staged":{"type":"boolean","description":"Show staged changes"}}}),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "GitCommit".to_string(),
            description: "Record changes to the repository with a message. Can stage specific files first.".to_string(),
            input_schema: json!({"type":"object","properties":{"message":{"type":"string"},"files":{"type":"array","items":{"type":"string"}},"working_dir":{"type":"string"}},"required":["message"]}),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "GitLog".to_string(),
            description: "Show commit logs with author, date, and message.".to_string(),
            input_schema: json!({"type":"object","properties":{"limit":{"type":"integer","description":"Max number of commits (default: 20)"},"working_dir":{"type":"string"}}}),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "GitBranch".to_string(),
            description: "List all branches or create a new branch.".to_string(),
            input_schema: json!({"type":"object","properties":{"name":{"type":"string","description":"New branch name"},"checkout":{"type":"boolean","description":"Checkout after create"},"working_dir":{"type":"string"}}}),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "GitCheckout".to_string(),
            description: "Switch to a specified branch.".to_string(),
            input_schema: json!({"type":"object","properties":{"name":{"type":"string","description":"Branch name to switch to"},"working_dir":{"type":"string"}},"required":["name"]}),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "GitStash".to_string(),
            description: "Save local changes to stash stack for later recovery.".to_string(),
            input_schema: json!({"type":"object","properties":{"working_dir":{"type":"string"}}}),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "GitAdd".to_string(),
            description: "Stage files for commit.".to_string(),
            input_schema: json!({"type":"object","properties":{"files":{"type":"array","items":{"type":"string"},"description":"Files to stage"},"working_dir":{"type":"string"}},"required":["files"]}),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "GitReset".to_string(),
            description: "Unstage files (reset HEAD).".to_string(),
            input_schema: json!({"type":"object","properties":{"files":{"type":"array","items":{"type":"string"},"description":"Files to unstage"},"working_dir":{"type":"string"}},"required":["files"]}),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "NotebookEdit".to_string(),
            description: "Edit a Jupyter notebook cell by index.".to_string(),
            input_schema: json!({"type":"object","properties":{"file_path":{"type":"string"},"cell_index":{"type":"integer"},"source":{"type":"array","items":{"type":"string"}}},"required":["file_path","cell_index"]}),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "ScheduleCron".to_string(),
            description: "Schedule a recurring task using cron-like syntax.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Unique identifier"},
                    "schedule": {"type": "string", "description": "Cron expression (e.g., '0 * * * *')"},
                    "task": {"type": "string", "description": "Task/command to execute"},
                    "enabled": {"type": "boolean", "default": true}
                },
                "required": ["name", "schedule", "task"]
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "AskUserQuestion".to_string(),
            description: "Ask the user one or more questions interactively when clarification is needed.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "questions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "question": {"type": "string"},
                                "header": {"type": "string"},
                                "options": {"type": "array", "items": {"type": "object", "properties": {"label": {"type": "string"}, "description": {"type": "string"}}, "required": ["label", "description"]}},
                                "multiSelect": {"type": "boolean"}
                            },
                            "required": ["question"]
                        }
                    }
                },
                "required": ["questions"]
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "ToolSearch".to_string(),
            description: "Search across available tools to find tools matching a query. Used when needing a specific capability.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Search query for finding relevant tools"},
                    "max_results": {"type": "integer", "description": "Maximum results (default: 10)"}
                },
                "required": ["query"]
            }),
            category: None,
            tags: Vec::new(),
        },

        // ===== Chrome DevTools 浏览器操作类 =====
        ToolDefinition {
            name: "BrowserDetect".to_string(),
            description: "Auto-detect all installed browsers (Chrome, Edge) on this machine. Returns browser names, paths, and versions. Should be called first before any other browser operation.".to_string(),
            input_schema: json!({"type":"object","properties":{}}),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "BrowserLaunch".to_string(),
            description: "Launch Chrome/Edge with remote debugging enabled. Auto-detects browser path if not specified. Opens with an isolated user data directory to avoid conflicts with existing browser instances. Returns the debugging port number.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "browser_path": {"type": "string", "description": "Full path to browser executable (optional, auto-detect if omitted)"},
                    "port": {"type": "integer", "description": "Remote debugging port (default: 9222)"},
                    "headless": {"type": "boolean", "description": "Run in headless mode (default: false)"}
                },
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "BrowserNavigate".to_string(),
            description: "Navigate a browser tab to a specific URL. Requires the browser to be launched with debugging port first.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {"type": "string", "description": "URL to navigate to"},
                    "port": {"type": "integer", "description": "Debugging port (default: 9222)"}
                },
                "required": ["url"]
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "BrowserGetContent".to_string(),
            description: "Get the current page content (text extracted from DOM) of the active browser tab. Useful for reading page text, forms, links, etc.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "port": {"type": "integer", "description": "Debugging port (default: 9222)"}
                },
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "BrowserScreenshot".to_string(),
            description: "Take a screenshot of the current browser tab. Returns base64-encoded PNG image that can be displayed or analyzed.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "port": {"type": "integer", "description": "Debugging port (default: 9222)"},
                    "format": {"type": "string", "enum": ["png", "jpeg"], "description": "Image format (default: png)"}
                },
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "BrowserClick".to_string(),
            description: "Click a DOM element on the page using CSS selector. Useful for clicking buttons, links, checkboxes, etc.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {"type": "string", "description": "CSS selector for the element to click (e.g., '#submit-btn', '.login-button', 'a[href=\"/next\"]')"},
                    "port": {"type": "integer", "description": "Debugging port (default: 9222)"}
                },
                "required": ["selector"]
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "BrowserFillInput".to_string(),
            description: "Fill text into an input field using CSS selector. Uses native setter to trigger React/Vue change events properly.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {"type": "string", "description": "CSS selector for the input element (e.g., '#username', 'input[name=\"email\"]')"},
                    "value": {"type": "string", "description": "Text value to fill into the input"},
                    "port": {"type": "integer", "description": "Debugging port (default: 9222)"}
                },
                "required": ["selector", "value"]
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "BrowserExecuteJs".to_string(),
            description: "Execute arbitrary JavaScript code in the browser page context and return the result. Powerful for complex interactions, data extraction, or DOM manipulation.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "script": {"type": "string", "description": "JavaScript code to execute (return values will be serialized as JSON)"},
                    "port": {"type": "integer", "description": "Debugging port (default: 9222)"}
                },
                "required": ["script"]
            }),
            category: None,
            tags: Vec::new(),
        },

        // ===== UI 自动化操作类（claw-automatically 集成）=====
        ToolDefinition {
            name: "ExecuteAutomation".to_string(),
            description: "Execute a natural language automation instruction on the user's desktop using CUA (Computer Use Agent). This tool can handle COMPLEX multi-step tasks that require visual analysis and interaction, such as: sending messages on WeChat/DingTalk, filling forms in applications, navigating UI to find and click elements, etc. It works by taking screenshots, analyzing them with vision AI, and performing mouse/keyboard actions step by step. ALSO handles simple tasks like opening applications. Examples: '给张三发微信消息：你在干什么？', '打开记事本并输入Hello World', '在Chrome中搜索Rust编程', '打开Word并新建文档'.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "instruction": {
                        "type": "string",
                        "description": "Natural language instruction describing the automation action to perform. Should be specific and actionable."
                    }
                },
                "required": ["instruction"]
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "CaptureScreen".to_string(),
            description: "Capture the current screen and return OCR text of what's visible. Returns plain text extracted from the screen via OCR — you can READ what's on screen. ALWAYS call this first before any automation action to understand the current desktop state. No parameters needed.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "OcrRecognizeScreen".to_string(),
            description: "Perform OCR on the current screen to extract all visible text. Returns the same OCR text as CaptureScreen. Use this when you need to re-read the screen after an action. For clicking specific elements, use CaptureScreen first to read text, then MouseClick with coordinates.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "language": {
                        "type": "string",
                        "description": "OCR language code (default: 'chi_sim+eng' for Chinese+English)",
                        "default": "chi_sim+eng"
                    }
                }
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "MouseClick".to_string(),
            description: "Simulate a mouse click at specified screen coordinates (x, y). Use when you need to click at a specific position on screen after OCR or screen analysis.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "x": {"type": "number", "description": "X coordinate on screen"},
                    "y": {"type": "number", "description": "Y coordinate on screen"}
                },
                "required": ["x", "y"]
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "MouseDoubleClick".to_string(),
            description: "Simulate a mouse double-click at specified screen coordinates. Commonly used for opening files or applications from desktop/icons.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "x": {"type": "number", "description": "X coordinate on screen"},
                    "y": {"type": "number", "description": "Y coordinate on screen"}
                },
                "required": ["x", "y"]
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "MouseRightClick".to_string(),
            description: "Simulate a mouse right-click at specified screen coordinates. Opens context menus on desktop and in applications.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "x": {"type": "number", "description": "X coordinate on screen"},
                    "y": {"type": "number", "description": "Y coordinate on screen"}
                },
                "required": ["x", "y"]
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "KeyboardType".to_string(),
            description: "Simulate keyboard text input. Types the given text character by character as if a user were typing on keyboard.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "text": {"type": "string", "description": "Text string to type"}
                },
                "required": ["text"]
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "KeyboardPress".to_string(),
            description: "Simulate pressing a single key or key combination. Examples: 'Enter', 'Tab', 'Escape', 'Ctrl+C', 'Alt+F4'. Use for special keys that cannot be typed as regular text.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "key": {"type": "string", "description": "Key name or combination (e.g., 'Enter', 'Ctrl+C', 'Alt+F4')"}
                },
                "required": ["key"]
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "ListInstalledApps".to_string(),
            description: "List installed applications on the user's system. On Windows, reads the registry for installed programs. On macOS, scans /Applications and uses Spotlight. On Linux, parses .desktop files. Returns application names, paths, and launch commands. Use this to find what software is available before launching it. Supports optional filter parameter to search for specific apps by name.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "filter": {
                        "type": "string",
                        "description": "Optional filter to search for applications by name (case-insensitive partial match). If omitted, returns all installed applications."
                    }
                }
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "LaunchApplication".to_string(),
            description: "Launch an installed application by name. Searches the system app registry first, then falls back to PATH lookup. PREFERRED way to open applications — use this instead of KeyboardPress+KeyboardType combo. Examples: 'Chrome', 'Visual Studio Code', 'WeChat', 'Notepad', 'QClaw'. Use ListInstalledApps to find available apps.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the application to launch (must match an installed application name). Use ListInstalledApps first if unsure of the exact name."
                    }
                },
                "required": ["name"]
            }),
            category: None,
            tags: Vec::new(),
        },

        // ===== 动态扩展创建类 =====
        ToolDefinition {
            name: "CreateTool".to_string(),
            description: "Create a new extension tool dynamically. The tool will be saved to the extensions directory and registered immediately. Use when you identify a recurring task pattern that would benefit from a dedicated tool. The tool definition includes name, description, input schema, and an optional handler script.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Tool name in snake_case (e.g., 'pdf_converter')"},
                    "description": {"type": "string", "description": "Clear description of what the tool does and when to use it"},
                    "input_schema": {"type": "object", "description": "JSON Schema defining the tool's input parameters", "properties": {}, "additionalProperties": true},
                    "handler_script": {"type": "string", "description": "Optional bash/python script that implements the tool logic. The script receives inputs as JSON via stdin and should output JSON to stdout."},
                    "save_to_project": {"type": "boolean", "description": "Deprecated. Tools are always saved to the app extensions directory (dev: .temp_build/extensions/, production: <install_dir>/extensions/).", "default": false}
                },
                "required": ["name", "description", "input_schema"]
            }),
            category: None,
            tags: Vec::new(),
        },
        ToolDefinition {
            name: "CreateSkill".to_string(),
            description: "Create a new skill dynamically. The skill will be saved as a SKILL.md file and registered immediately. Use when you identify a workflow or domain expertise that should be encapsulated as a reusable skill. Skills are triggered by matching user intent to the skill's description.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Skill name in kebab-case (e.g., 'api-doc-generator')"},
                    "description": {"type": "string", "description": "Comprehensive description including what the skill does and when to use it. This is the primary trigger mechanism."},
                    "when_to_use": {"type": "string", "description": "Specific scenarios when this skill should be triggered"},
                    "allowed_tools": {"type": "array", "items": {"type": "string"}, "description": "List of tool names this skill is allowed to use"},
                    "instructions": {"type": "string", "description": "The skill's Markdown body — detailed instructions for using the skill"},
                    "save_to_project": {"type": "boolean", "description": "Deprecated. Skills are always saved to the app skills directory (dev: .temp_build/skills/, production: <install_dir>/skills/).", "default": false}
                },
                "required": ["name", "description", "instructions"]
            }),
            category: None,
            tags: Vec::new(),
        },
    ]
}
