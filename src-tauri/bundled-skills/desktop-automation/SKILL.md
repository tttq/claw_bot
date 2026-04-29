---
name: desktop-automation
description: "Control the user's desktop operating system — open applications, click UI elements, type text, read screen content, and automate multi-step workflows. Use when: (1) user asks to open/launch an app, (2) user wants to click a button or interact with desktop UI, (3) user needs to read what's on screen, (4) user wants to automate a repetitive desktop task, (5) user asks to fill in forms or type text into applications. NOT for: web browsing (use browser tools), file operations (use file tools), or terminal commands (use shell tools). Requires: UI Automation enabled in Settings > Tools."
when_to_use: Desktop interaction tasks requiring screen reading, mouse/keyboard control, or application automation
allowed-tools: ["ExecuteAutomation", "CaptureScreen", "OcrRecognizeScreen", "MouseClick", "MouseDoubleClick", "MouseRightClick", "KeyboardType", "KeyboardPress", "ListInstalledApps", "LaunchApplication"]
argument_hint: <desktop task description, e.g. "open Chrome", "click the Save button", "read the error dialog">
user_invocable: true
version: 2.0.0
model: claude-sonnet-4
effort: medium
---

# Desktop Automation

Control the user's desktop through screen capture, OCR text recognition, and mouse/keyboard input simulation.

## Available Tools

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| `ExecuteAutomation` | One-shot natural language instruction — handles full pipeline (capture → OCR → LLM decide → act) automatically | `instruction` (string) |
| `ListInstalledApps` | List installed applications on the system. Windows: reads registry; macOS: scans /Applications; Linux: parses .desktop files | `filter` (optional, partial name match) |
| `LaunchApplication` | Launch an installed application by name. Finds the app via system registry and starts it directly | `name` (string, app name) |
| `CaptureScreen` | Take a screenshot and read screen text via OCR (returns text summary) | (no params) |
| `OcrRecognizeScreen` | Read all text and UI elements on screen with bounding box coordinates | `language` (optional, default: chi_sim+eng) |
| `MouseClick` | Left-click at coordinates | `x`, `y` (pixels from top-left of primary screen) |
| `MouseDoubleClick` | Double-click at coordinates (for opening files/icons) | `x`, `y` |
| `MouseRightClick` | Right-click at coordinates (for context menus) | `x`, `y` |
| `KeyboardType` | Type a string of text character by character | `text` |
| `KeyboardPress` | Press a special key or key combination | `key` (e.g. Enter, Tab, Escape, Super, Control, Alt, F1-F12) |

## How It Works Internally

The automation engine (`claw-automatically`) implements a full pipeline:

1. **Screen Capture** — Uses FFmpeg to grab the desktop frame
   - Windows: `gdigrab` device → captures "desktop"
   - Linux: `x11grab` device → captures `:0.0` display
   - macOS: `avfoundation` device → captures screen index "1"
2. **Image Preprocessing** — Converts to grayscale, applies threshold for better OCR accuracy
3. **OCR Recognition** — Uses Tesseract-compatible engine to extract text with bounding boxes
4. **Spatial Analysis** — Clusters OCR results into UI regions, classifies elements (Button/InputField/Link/Icon/Text), groups by row
5. **LLM Decision** — Sends screen structure + user instruction to LLM, receives JSON operation command
6. **Input Execution** — Simulates mouse/keyboard via platform-native APIs
7. **Verification** — Optionally re-captures screen to confirm action succeeded

## Two Modes of Operation

### Quick Mode — `ExecuteAutomation`

For simple tasks like "open Chrome" or "click Save", use `ExecuteAutomation` with a descriptive instruction. It handles the full pipeline automatically in one call.

```json
{ "instruction": "打开桌面上的 Chrome 浏览器" }
```

```json
{ "instruction": "双击桌面上的 QClaw 图标" }
```

**When to use Quick Mode:**
- Single-step actions (open an app, click a button, type text)
- User's intent is clear and unambiguous
- You don't need to inspect intermediate screen state

### Step-by-Step Mode — Manual Tool Chain

For complex or multi-step tasks, manually call individual tools for precise control and verification at each step:

1. **See** — `CaptureScreen` to get a screenshot
2. **Understand** — `OcrRecognizeScreen` to get structured UI element data with coordinates
3. **Act** — Use the appropriate input tool based on OCR results
4. **Verify** — `CaptureScreen` again to confirm the action succeeded
5. **Repeat** — Continue until the task is complete

**When to use Step-by-Step Mode:**
- Multi-step workflows requiring verification between steps
- You need to read screen content and make decisions
- Previous step's result affects the next action
- Quick mode failed and you need more control

## Platform-Specific Operations

### 🪟 Windows

**Open an Application (Recommended — Direct Launch):**
```
1. LaunchApplication  → name: "Chrome"       (directly launch by name from registry)
2. CaptureScreen      →                       (verify it opened)
```

**Open an Application (Search — if LaunchApplication fails):**
```
1. KeyboardPress  → key: "Super"           (open Start menu)
2. KeyboardType   → text: "Chrome"         (type app name in search)
3. KeyboardPress  → key: "Enter"           (launch the app)
4. CaptureScreen  →                        (verify it opened)
```

**Find and Open an Unknown Application:**
```
1. ListInstalledApps  → filter: "photo"    (search for photo editing apps)
2. LaunchApplication  → name: "Photoshop"  (launch the found app)
3. CaptureScreen      →                     (verify)
```

**Open a Desktop Shortcut / File:**
```
1. OcrRecognizeScreen  →                   (find the icon on desktop)
2. MouseDoubleClick     → x: 150, y: 350   (double-click the icon)
3. CaptureScreen        →                   (verify it opened)
```

**Open File via Explorer:**
```
1. KeyboardPress  → key: "Super"
2. KeyboardType   → text: "explorer"        (open File Explorer)
3. KeyboardPress  → key: "Enter"
4. KeyboardType   → text: "C:\Users\user\Documents\report.docx"  (type path in address bar)
5. KeyboardPress  → key: "Enter"
```

**Right-click Context Menu:**
```
1. OcrRecognizeScreen  →                   (find the target element)
2. MouseRightClick      → x: 500, y: 300   (right-click to open context menu)
```

**Key names for Windows:**
- `Super` — Windows key (open Start menu)
- `Control` / `Ctrl` — Ctrl key
- `Alt` — Alt key
- `Tab` — Switch between windows (Alt+Tab)
- `F4` — Close window (Alt+F4)
- `Escape` — Cancel/close dialog

### 🐧 Linux

**Open an Application (Recommended — Direct Launch):**
```
1. LaunchApplication  → name: "Firefox"     (directly launch by name from .desktop files)
2. CaptureScreen      →                       (verify)
```

**Open an Application (Search — if LaunchApplication fails):**
```
1. KeyboardPress  → key: "Super"           (open Activities/launcher)
2. KeyboardType   → text: "firefox"        (type app name)
3. KeyboardPress  → key: "Enter"           (launch)
4. CaptureScreen  →                        (verify)
```

**Open from Terminal (alternative):**
```
1. KeyboardPress  → key: "Control"         (hold Ctrl)
2. KeyboardPress  → key: "Alt"             (hold Alt)
3. KeyboardPress  → key: "T"               (open terminal — Ctrl+Alt+T)
4. KeyboardType   → text: "firefox &"      (type command)
5. KeyboardPress  → key: "Enter"
```

**Key names for Linux:**
- `Super` — Super/Meta key (open Activities overview)
- `Control` / `Ctrl` — Ctrl key
- `Alt` — Alt key
- `Tab` — Window switcher (Alt+Tab)
- `Escape` — Cancel

**Note:** Linux uses X11 XTest extension for input simulation. Requires X11 display server (Wayland not supported — use XWayland).

### 🍎 macOS

**Open an Application (Recommended — Direct Launch):**
```
1. LaunchApplication  → name: "Safari"      (directly launch by name from /Applications)
2. CaptureScreen      →                       (verify)
```

**Open an Application (Search — if LaunchApplication fails):**
```
1. KeyboardPress  → key: "Super"           (open Spotlight — Command+Space)
2. KeyboardType   → text: "Safari"         (type app name)
3. KeyboardPress  → key: "Enter"           (launch)
4. CaptureScreen  →                        (verify)
```

**Open from Finder:**
```
1. KeyboardPress  → key: "Super"           (Spotlight)
2. KeyboardType   → text: "Finder"
3. KeyboardPress  → key: "Enter"
4. KeyboardPress  → key: "Super"           (Spotlight again)
5. KeyboardType   → text: "Documents"      (navigate to folder)
6. KeyboardPress  → key: "Enter"
```

**Key names for macOS:**
- `Super` — Command (⌘) key
- `Control` — Control key
- `Alt` — Option (⌥) key
- `Tab` — Command+Tab app switcher
- `Escape` — Cancel
- `F11` — Show desktop

**Note:** macOS requires Accessibility permissions for the app. If input simulation doesn't work, guide the user to System Settings → Privacy & Security → Accessibility → enable the app.

## Common Task Patterns

### Click a UI Element (All Platforms)
```
1. OcrRecognizeScreen  →                   (find element coordinates from bbox)
2. MouseClick           → x: 540, y: 320   (click at element center: (x1+x2)/2, (y1+y2)/2)
3. CaptureScreen        →                   (verify result)
```

### Fill a Form (All Platforms)
```
1. OcrRecognizeScreen  →                   (find input fields)
2. MouseClick           → x: 300, y: 200   (click Name field)
3. KeyboardType         → text: "John"     (type name)
4. KeyboardPress        → key: "Tab"       (move to next field)
5. KeyboardType         → text: "john@email.com"
6. MouseClick           → x: 400, y: 500   (click Submit button)
```

### Read a Dialog/Error Message (All Platforms)
```
1. CaptureScreen         →                 (get screenshot)
2. OcrRecognizeScreen   →                 (extract all text)
   → Report the text content to the user
```

### Close a Window (All Platforms)
```
1. OcrRecognizeScreen  →                   (find close button position)
2. MouseClick           → x: ..., y: ...   (click the X/Close button)
```

### Switch Between Windows (All Platforms)
```
1. KeyboardPress  → key: "Alt"             (hold modifier)
2. KeyboardPress  → key: "Tab"             (press Tab to cycle)
3. Release keys                          (release to switch)
```
Or simply use `ExecuteAutomation` with instruction "switch to the Chrome window".

## OCR Result Format

The `OcrRecognizeScreen` tool returns structured JSON:

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
          "bbox": {"x1": 500, "y1": 500, "x2": 580, "y2": 540},
          "row_number": 5,
          "nearby_title": "File Options"
        }
      ]
    }
  ]
}
```

**Element types:** `Text`, `Button`, `InputField`, `Link`, `Icon`, `Unknown`

**How to calculate click coordinates from bbox:**
- Center X = `(x1 + x2) / 2`
- Center Y = `(y1 + y2) / 2`

**Example:** For `"bbox": {"x1": 500, "y1": 500, "x2": 580, "y2": 540}` → click at `(540, 520)`

## Supported Key Names

| Key | Name to use |
|-----|-------------|
| Enter/Return | `Enter` or `Return` |
| Tab | `Tab` |
| Backspace | `Backspace` or `BS` |
| Delete | `Delete` or `Del` |
| Escape | `Escape` or `Esc` |
| Space | `Space` |
| Arrow keys | `Left`, `Right`, `Up`, `Down` |
| Function keys | `F1` through `F12` |
| Windows/Super | `Super` |
| Control | `Control` or `Ctrl` |
| Alt/Option | `Alt` |
| Shift | `Shift` |

**Unicode text:** Chinese/Japanese/Korean characters are supported via Unicode input on Windows, and via clipboard paste on Linux/macOS.

## Important Rules

- **ALWAYS capture/OCR first** — Never guess coordinates. Always read the screen before clicking.
- **Verify after acting** — After each action, capture the screen to confirm it worked.
- **Use OCR coordinates** — Click at the center of the bounding box: `((x1+x2)/2, (y1+y2)/2)`.
- **Wait between steps** — After clicking or typing, the UI may need a moment to respond. If verification shows no change, wait and retry.
- **Handle errors gracefully** — If an action fails, re-capture the screen, analyze what changed, and try an alternative approach.
- **Prefer Quick Mode first** — Try `ExecuteAutomation` for simple tasks. Only fall back to step-by-step if it fails or you need more control.
- **Respect user confirmation** — For destructive actions (deleting files, closing unsaved work), use `[CONFIRM_REQUIRED]` signal.
- **Don't over-automate** — If the task can be done more efficiently with shell commands or file tools, prefer those over UI automation.
- **Coordinate system** — All coordinates are in pixels from the top-left corner (0,0) of the primary monitor.
- **Double-click for opening** — Use `MouseDoubleClick` to open desktop icons, files, and folders. Use `MouseClick` for buttons and links.
