// Claw Desktop - HTML净化工具 - 清理HTML中的危险脚本和事件处理器，防止XSS攻击
export function sanitizeHtml(html: string): string {
  return html
    .replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, '')
    .replace(/\bon\w+\s*=\s*"[^"]*"/gi, '')
    .replace(/\bon\w+\s*=\s*'[^']*'/gi, '')
    .replace(/\bon\w+\s*=\s*[^\s>]+/gi, '')
    .replace(/javascript:/gi, '')
    .replace(/vbscript:/gi, '')
    .replace(/data:\s*text\/html/gi, '')
}