---
name: desktop-automation
description: "Control the user's desktop operating system — open applications, click UI elements, type text, read screen content, and automate multi-step workflows. Use when: (1) user asks to open/launch an app, (2) user wants to click a button or interact with desktop UI, (3) user needs to read what's on screen, (4) user wants to automate a repetitive desktop task, (5) user asks to fill in forms or type text into applications. NOT for: web browsing (use browser tools), file operations (use file tools), or terminal commands (use shell tools). Requires: UI Automation enabled in Settings > Tools."
when_to_use: Desktop interaction tasks requiring screen reading, mouse/keyboard control, or application automation
allowed-tools: ["ExecuteAutomation", "CaptureScreen", "OcrRecognizeScreen", "MouseClick", "MouseDoubleClick", "KeyboardType", "KeyboardPress"]
argument_hint: <desktop task description, e.g. "open Chrome", "click the Save button", "read the error dialog">
user_invocable: true
version: 1.0.0
model: claude-sonnet-4
effort: medium
---

# Desktop Automation

Control the user's desktop through screen capture, OCR, and input simulation.

## Available Tools

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| `execute_automation` | One-shot natural language instruction — handles full pipeline automatically | `instruction` (string) |
| `capture_screen` | Take a screenshot of the current desktop | (no params) |
| `ocr_recognize_screen` | Read all text and UI elements on screen | `language` (optional, default: chi_sim+eng) |
| `mouse_click` | Left-click at coordinates | `x`, `y` (pixels from top-left) |
| `mouse_double_click` | Double-click at coordinates | `x`, `y` |
| `keyboard_type` | Type a string of text | `text` |
| `keyboard_press` | Press a special key | `key` (e.g. Enter, Tab, Escape, Super, Alt, Control) |

## Workflow

### Quick Mode — Single Instruction

For simple tasks like "open Chrome" or "click Save", use `execute_automation` with a descriptive instruction. It handles the full pipeline (capture → OCR → decide → act) automatically.

```
Tool: execute_automation
Input: { "instruction": "打开桌面上的 Chrome 浏览器" }
```

### Step-by-Step Mode — Precise Control

For complex or multi-step tasks, follow this loop:

1. **See** — `capture_screen` to get a screenshot
2. **Understand** — `ocr_recognize_screen` to get structured UI element data (text, buttons, coordinates)
3. **Act** — Use the appropriate input tool based on OCR results:
   - Click a button/icon → `mouse_click(x, y)` where x,y come from OCR bounding box center
   - Open an app → `keyboard_press("Super")` → `keyboard_type("app name")` → `keyboard_press("Enter")`
   - Type into a field → `mouse_click(x, y)` on the field first, then `keyboard_type("text")`
   - Navigate menus → sequence of `mouse_click` or `keyboard_press("Tab")` / `keyboard_press("Enter")`
4. **Verify** — `capture_screen` again to confirm the action succeeded
5. **Repeat** — Continue until the task is complete

## Common Patterns

### Open an Application (Windows/Linux/macOS)

```
1. keyboard_press  → key: "Super"          (open Start/launcher)
2. keyboard_type   → text: "Chrome"        (type app name)
3. keyboard_press  → key: "Enter"          (launch)
4. capture_screen  →                       (verify it opened)
```

### Click a UI Element

```
1. ocr_recognize_screen  →                 (find element coordinates)
2. mouse_click           → x: 540, y: 320  (click at element center)
3. capture_screen        →                 (verify result)
```

### Fill a Form

```
1. ocr_recognize_screen  →                 (find input fields)
2. mouse_click           → x: 300, y: 200  (click Name field)
3. keyboard_type         → text: "John"    (type name)
4. keyboard_press        → key: "Tab"      (move to next field)
5. keyboard_type         → text: "john@email.com" (type email)
6. mouse_click           → x: 400, y: 500  (click Submit)
```

### Read a Dialog/Error Message

```
1. capture_screen         →                (get screenshot)
2. ocr_recognize_screen   →                (extract all text)
   → Report the text content to the user
```

### Close a Window/Dialog

```
1. ocr_recognize_screen  →                 (find close button or confirm position)
2. mouse_click           → x: ..., y: ...  (click Close/OK/Cancel)
```

## Important Rules

- **ALWAYS capture/OCR first** — Never guess coordinates. Always read the screen before clicking.
- **Verify after acting** — After each action, capture the screen to confirm it worked.
- **Use OCR coordinates** — The `ocr_recognize_screen` result includes bounding boxes with `x1,y1,x2,y2`. Click at the center: `((x1+x2)/2, (y1+y2)/2)`.
- **Small delays between steps** — After clicking or typing, the UI may need a moment to respond. If verification shows no change, wait and retry.
- **Handle errors gracefully** — If an action fails, re-capture the screen, analyze what changed, and try an alternative approach.
- **Respect user confirmation** — For destructive actions (deleting files, closing unsaved work), use `[CONFIRM_REQUIRED]` signal to ask the user first.
- **Don't over-automate** — If the task can be done more efficiently with shell commands or file tools, prefer those over UI automation.

## OCR Result Format

The `ocr_recognize_screen` tool returns structured data:

```json
{
  "screen_size": [1920, 1080],
  "regions": [
    {
      "id": "region-1",
      "title": "Dialog",
      "bbox": {"x1": 300, "y1": 200, "x2": 800, "y2": 600},
      "elements": [
        {
          "id": "elem-1",
          "text": "Save",
          "element_type": "Button",
          "bbox": {"x1": 500, "y1": 500, "x2": 580, "y2": 540}
        }
      ]
    }
  ]
}
```

Element types: `Text`, `Button`, `InputField`, `Link`, `Icon`, `Unknown`

## Platform Notes

- **Windows**: Uses Win32 SendInput API for mouse/keyboard
- **Linux**: Uses X11 XTest extension
- **macOS**: Uses CoreGraphics CGEvent
- **Screen capture**: Uses FFmpeg (gdigrab on Windows, x11grab on Linux, avfoundation on macOS)
