'use client'

import { useEditor, EditorContent } from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'
import Image from '@tiptap/extension-image'
import Link from '@tiptap/extension-link'
import Underline from '@tiptap/extension-underline'
import TextAlign from '@tiptap/extension-text-align'
import Placeholder from '@tiptap/extension-placeholder'
import { CodeBlockLowlight } from '@tiptap/extension-code-block-lowlight'
import MarkdownIt from 'markdown-it'
import { createLowlight, all } from 'lowlight'
import { useCallback, useRef, useState } from 'react'
import { cn } from '@/lib/utils'
import {
  Bold, Italic, Underline as UnderlineIcon, Strikethrough,
  Heading1, Heading2, Heading3,
  List, ListOrdered, Quote, Code, Code2,
  Link2, ImageIcon, AlignLeft, AlignCenter, AlignRight,
  Minus, Undo2, Redo2, Upload,
} from 'lucide-react'

const lowlight = createLowlight(all)
const markdown = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true,
})

function normalizeClipboardText(text: string): string {
  return text.replace(/\r\n?/g, '\n')
}

function looksLikeMarkdown(text: string): boolean {
  const normalized = normalizeClipboardText(text).trim()
  if (!normalized) return false

  return [
    /^#{1,6}\s+/m,
    /^\s*[-*+]\s+/m,
    /^\s*\d+\.\s+/m,
    /^\s*>\s+/m,
    /^\s*```[\s\S]*```\s*$/m,
    /`[^`]+`/,
    /\*\*[^*\n]+\*\*/,
    /__[^_\n]+__/,
    /\[[^\]]+\]\([^\)]+\)/,
    /!\[[^\]]*\]\([^\)]+\)/,
    /^\s*(---|\*\*\*|___)\s*$/m,
    /^\s*\|.+\|\s*$/m,
  ].some((pattern) => pattern.test(normalized))
}

function clipboardHtmlHasSemanticFormatting(html: string): boolean {
  const normalized = html.trim()
  if (!normalized || typeof window === 'undefined') return false

  const doc = new DOMParser().parseFromString(normalized, 'text/html')
  const meaningfulSelectors = [
    'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
    'ul', 'ol', 'li', 'blockquote', 'a[href]', 'img',
    'table', 'thead', 'tbody', 'tr', 'th', 'td',
    'strong', 'b', 'em', 'i', 'hr', 'code',
  ]

  return meaningfulSelectors.some((selector) => doc.body.querySelector(selector) !== null)
}

// ── Toolbar button ──────────────────────────────────────────────────────────
function ToolbarBtn({
  onClick,
  active,
  disabled,
  title,
  children,
}: {
  onClick: () => void
  active?: boolean
  disabled?: boolean
  title: string
  children: React.ReactNode
}) {
  return (
    <button
      type="button"
      title={title}
      disabled={disabled}
      onMouseDown={(e) => {
        e.preventDefault() // keep editor focus
        onClick()
      }}
      className={cn(
        'inline-flex items-center justify-center w-7 h-7 rounded text-sm transition-colors',
        'disabled:opacity-30 disabled:cursor-not-allowed',
        active
          ? 'bg-accent/20 text-accent'
          : 'text-muted-foreground hover:text-foreground hover:bg-muted',
      )}
    >
      {children}
    </button>
  )
}

function Sep() {
  return <div className="w-px h-5 bg-border mx-0.5 shrink-0" />
}

// ── Link dialog ─────────────────────────────────────────────────────────────
function LinkDialog({
  onConfirm,
  onCancel,
  initial,
}: {
  onConfirm: (url: string) => void
  onCancel: () => void
  initial: string
}) {
  const [val, setVal] = useState(initial)
  return (
    <div className="flex items-center gap-2 px-2 py-1.5 bg-popover border border-border rounded-lg shadow-lg">
      <input
        autoFocus
        type="url"
        value={val}
        onChange={(e) => setVal(e.target.value)}
        placeholder="https://..."
        className="text-sm bg-transparent outline-none text-foreground placeholder:text-muted-foreground w-64"
        onKeyDown={(e) => {
          if (e.key === 'Enter') onConfirm(val)
          if (e.key === 'Escape') onCancel()
        }}
      />
      <button
        type="button"
        onMouseDown={(e) => { e.preventDefault(); onConfirm(val) }}
        className="text-xs px-2 py-0.5 bg-accent text-accent-foreground rounded"
      >
        Set
      </button>
      <button
        type="button"
        onMouseDown={(e) => { e.preventDefault(); onCancel() }}
        className="text-xs px-2 py-0.5 bg-muted text-muted-foreground rounded"
      >
        Cancel
      </button>
    </div>
  )
}

// ── Main editor ─────────────────────────────────────────────────────────────
interface RichEditorProps {
  content?: string
  onChange?: (html: string) => void
  placeholder?: string
  minHeight?: number
  uploadImage?: (file: File) => Promise<string>
}

export function RichEditor({
  content = '',
  onChange,
  placeholder = 'Start writing…',
  minHeight = 400,
  uploadImage,
}: RichEditorProps) {
  const [showLinkDialog, setShowLinkDialog] = useState(false)
  const fileInputRef = useRef<HTMLInputElement>(null)

  const editor = useEditor({
    immediatelyRender: false,
    extensions: [
      StarterKit.configure({
        codeBlock: false, // replaced by lowlight version
      }),
      Underline,
      TextAlign.configure({ types: ['heading', 'paragraph'] }),
      Placeholder.configure({ placeholder }),
      Image.configure({ allowBase64: true }),
      Link.configure({
        openOnClick: false,
        HTMLAttributes: { rel: 'noopener noreferrer', target: '_blank' },
      }),
      CodeBlockLowlight.configure({ lowlight }),
    ],
    content,
    editorProps: {
      attributes: {
        class: 'outline-none',
      },
      handlePaste(view, event) {
        const clipboard = event.clipboardData
        const plainText = clipboard?.getData('text/plain') ?? ''
        const htmlText = clipboard?.getData('text/html') ?? ''

        if (!editor || !plainText.trim()) return false
        if (editor.isActive('codeBlock') || editor.isActive('code')) return false
        if (!looksLikeMarkdown(plainText)) return false
        if (clipboardHtmlHasSemanticFormatting(htmlText)) return false

        const renderedHtml = markdown.render(normalizeClipboardText(plainText).trim())
        if (!renderedHtml.trim()) return false

        event.preventDefault()
        editor.chain().focus().insertContent(renderedHtml).run()
        return true
      },
    },
    onUpdate({ editor }) {
      onChange?.(editor.getHTML())
    },
  })

  // ── Toolbar actions ───────────────────────────────────────────────────────
  const insertLink = useCallback(() => {
    if (!editor) return
    setShowLinkDialog(true)
  }, [editor])

  const confirmLink = useCallback(
    (url: string) => {
      if (!editor) return
      setShowLinkDialog(false)
      if (!url) {
        editor.chain().focus().extendMarkToLink().unsetLink().run()
        return
      }
      const href = url.startsWith('http') ? url : `https://${url}`
      editor.chain().focus().extendMarkToLink().setLink({ href }).run()
    },
    [editor],
  )

  const insertImageFromFile = useCallback(
    async (file: File) => {
      if (!editor) return
      let src: string
      if (uploadImage) {
        src = await uploadImage(file)
      } else {
        // fallback: base64
        src = await new Promise((resolve) => {
          const reader = new FileReader()
          reader.onload = (e) => resolve(e.target?.result as string)
          reader.readAsDataURL(file)
        })
      }
      editor.chain().focus().setImage({ src }).run()
    },
    [editor, uploadImage],
  )

  const insertImageFromUrl = useCallback(() => {
    if (!editor) return
    const url = window.prompt('Image URL:')
    if (url) editor.chain().focus().setImage({ src: url }).run()
  }, [editor])

  if (!editor) return null

  const currentLink = editor.getAttributes('link').href ?? ''

  return (
    <div className="flex flex-col border border-border rounded-lg overflow-hidden bg-card">
      {/* Toolbar */}
      <div className="flex flex-wrap items-center gap-0.5 p-2 border-b border-border bg-muted/30">
        {/* History */}
        <ToolbarBtn title="Undo" onClick={() => editor.chain().focus().undo().run()} disabled={!editor.can().undo()}>
          <Undo2 className="w-3.5 h-3.5" />
        </ToolbarBtn>
        <ToolbarBtn title="Redo" onClick={() => editor.chain().focus().redo().run()} disabled={!editor.can().redo()}>
          <Redo2 className="w-3.5 h-3.5" />
        </ToolbarBtn>

        <Sep />

        {/* Headings */}
        <ToolbarBtn
          title="Heading 1"
          active={editor.isActive('heading', { level: 1 })}
          onClick={() => editor.chain().focus().toggleHeading({ level: 1 }).run()}
        >
          <Heading1 className="w-3.5 h-3.5" />
        </ToolbarBtn>
        <ToolbarBtn
          title="Heading 2"
          active={editor.isActive('heading', { level: 2 })}
          onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()}
        >
          <Heading2 className="w-3.5 h-3.5" />
        </ToolbarBtn>
        <ToolbarBtn
          title="Heading 3"
          active={editor.isActive('heading', { level: 3 })}
          onClick={() => editor.chain().focus().toggleHeading({ level: 3 }).run()}
        >
          <Heading3 className="w-3.5 h-3.5" />
        </ToolbarBtn>

        <Sep />

        {/* Inline marks */}
        <ToolbarBtn
          title="Bold"
          active={editor.isActive('bold')}
          onClick={() => editor.chain().focus().toggleBold().run()}
        >
          <Bold className="w-3.5 h-3.5" />
        </ToolbarBtn>
        <ToolbarBtn
          title="Italic"
          active={editor.isActive('italic')}
          onClick={() => editor.chain().focus().toggleItalic().run()}
        >
          <Italic className="w-3.5 h-3.5" />
        </ToolbarBtn>
        <ToolbarBtn
          title="Underline"
          active={editor.isActive('underline')}
          onClick={() => editor.chain().focus().toggleUnderline().run()}
        >
          <UnderlineIcon className="w-3.5 h-3.5" />
        </ToolbarBtn>
        <ToolbarBtn
          title="Strikethrough"
          active={editor.isActive('strike')}
          onClick={() => editor.chain().focus().toggleStrike().run()}
        >
          <Strikethrough className="w-3.5 h-3.5" />
        </ToolbarBtn>
        <ToolbarBtn
          title="Inline code"
          active={editor.isActive('code')}
          onClick={() => editor.chain().focus().toggleCode().run()}
        >
          <Code className="w-3.5 h-3.5" />
        </ToolbarBtn>

        <Sep />

        {/* Lists */}
        <ToolbarBtn
          title="Bullet list"
          active={editor.isActive('bulletList')}
          onClick={() => editor.chain().focus().toggleBulletList().run()}
        >
          <List className="w-3.5 h-3.5" />
        </ToolbarBtn>
        <ToolbarBtn
          title="Ordered list"
          active={editor.isActive('orderedList')}
          onClick={() => editor.chain().focus().toggleOrderedList().run()}
        >
          <ListOrdered className="w-3.5 h-3.5" />
        </ToolbarBtn>
        <ToolbarBtn
          title="Blockquote"
          active={editor.isActive('blockquote')}
          onClick={() => editor.chain().focus().toggleBlockquote().run()}
        >
          <Quote className="w-3.5 h-3.5" />
        </ToolbarBtn>
        <ToolbarBtn
          title="Code block"
          active={editor.isActive('codeBlock')}
          onClick={() => editor.chain().focus().toggleCodeBlock().run()}
        >
          <Code2 className="w-3.5 h-3.5" />
        </ToolbarBtn>

        <Sep />

        {/* Alignment */}
        <ToolbarBtn
          title="Align left"
          active={editor.isActive({ textAlign: 'left' })}
          onClick={() => editor.chain().focus().setTextAlign('left').run()}
        >
          <AlignLeft className="w-3.5 h-3.5" />
        </ToolbarBtn>
        <ToolbarBtn
          title="Align center"
          active={editor.isActive({ textAlign: 'center' })}
          onClick={() => editor.chain().focus().setTextAlign('center').run()}
        >
          <AlignCenter className="w-3.5 h-3.5" />
        </ToolbarBtn>
        <ToolbarBtn
          title="Align right"
          active={editor.isActive({ textAlign: 'right' })}
          onClick={() => editor.chain().focus().setTextAlign('right').run()}
        >
          <AlignRight className="w-3.5 h-3.5" />
        </ToolbarBtn>

        <Sep />

        {/* Link */}
        <ToolbarBtn
          title="Link"
          active={editor.isActive('link')}
          onClick={insertLink}
        >
          <Link2 className="w-3.5 h-3.5" />
        </ToolbarBtn>

        {/* Image */}
        <ToolbarBtn title="Insert image from URL" onClick={insertImageFromUrl}>
          <ImageIcon className="w-3.5 h-3.5" />
        </ToolbarBtn>
        <ToolbarBtn
          title="Upload image"
          onClick={() => fileInputRef.current?.click()}
        >
          <Upload className="w-3.5 h-3.5" />
        </ToolbarBtn>

        <Sep />

        {/* HR */}
        <ToolbarBtn
          title="Horizontal rule"
          onClick={() => editor.chain().focus().setHorizontalRule().run()}
        >
          <Minus className="w-3.5 h-3.5" />
        </ToolbarBtn>

        {/* Markdown import hint */}
        <div className="ml-auto text-[10px] text-muted-foreground font-mono hidden sm:block">
          Markdown shortcuts active
        </div>
      </div>

      {/* Link dialog */}
      {showLinkDialog && (
        <div className="p-2 border-b border-border bg-muted/20">
          <LinkDialog
            initial={currentLink}
            onConfirm={confirmLink}
            onCancel={() => setShowLinkDialog(false)}
          />
        </div>
      )}

      {/* Editor area */}
      <div
        className="blog-editor-content px-6 py-5 cursor-text"
        style={{ minHeight }}
        onClick={() => editor.commands.focus()}
      >
        <EditorContent editor={editor} />
      </div>

      {/* Hidden file input for image upload */}
      <input
        ref={fileInputRef}
        type="file"
        accept="image/*"
        className="hidden"
        onChange={(e) => {
          const file = e.target.files?.[0]
          if (file) insertImageFromFile(file)
          e.target.value = ''
        }}
      />

      {/* Footer: word count */}
      <div className="flex items-center justify-end px-4 py-1.5 border-t border-border bg-muted/20 text-[11px] text-muted-foreground">
        {editor.storage.characterCount?.words?.() ?? 0} words
      </div>
    </div>
  )
}
