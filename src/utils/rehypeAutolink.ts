// Claw Desktop - Rehype自动链接插件 - 将Markdown中的纯文本URL自动转换为可点击的超链接
import type { Plugin } from 'unified'
import type { Root, Element, RootContent } from 'hast'

const URL_REGEX = /https?:\/\/[^\s<>"')\]{}]+/g

export function rehypeAutolink(): Plugin<[], Root> {
  return () => {
    return (tree: Root) => {
      visit(tree)
    }
  }
}

function visit(tree: Root) {
  if (tree.type !== 'root') return

  for (const node of tree.children) {
    if (node.type === 'element') visitNode(node as Element)
  }
}

function visitNode(node: Element | Root) {
  if ('children' in node) {
    const children = node.children
    for (let i = children.length - 1; i >= 0; i--) {
      const child = children[i] as RootContent
      if (child.type === 'text') {
        const textVal = (child as { type: 'text'; value: string }).value
        if (typeof textVal !== 'string') continue
        URL_REGEX.lastIndex = 0
        if (URL_REGEX.test(textVal)) {
          URL_REGEX.lastIndex = 0
          const matches = Array.from(textVal.matchAll(URL_REGEX))
          if (matches.length > 0) {
            const newNodes: Array<{ type: 'text'; value: string } | Element> = []
            let lastIndex = 0

            for (const match of matches) {
              const url = match[0]
              const start = match.index!

              if (start > lastIndex) {
                newNodes.push({ type: 'text', value: textVal.slice(lastIndex, start) })
              }

              newNodes.push({
                type: 'element',
                tagName: 'a',
                properties: {
                  href: url,
                  target: '_blank',
                  rel: 'noopener noreferrer',
                },
                children: [{ type: 'text', value: url }],
              })

              lastIndex = start + url.length
            }

            if (lastIndex < textVal.length) {
              newNodes.push({ type: 'text', value: textVal.slice(lastIndex) })
            }

            children.splice(i, 1, ...(newNodes as RootContent[]))
          }
        }
      } else if (child.type === 'element') {
        visitNode(child as Element)
      }
    }
  }
}
