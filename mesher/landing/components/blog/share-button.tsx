'use client'

import { Button } from '@/components/ui/button'
import { Share2 } from 'lucide-react'

export function ShareButton({ title }: { title: string }) {
  return (
    <Button
      variant="ghost"
      size="sm"
      className="gap-2 text-muted-foreground"
      onClick={() => {
        if (typeof navigator !== 'undefined') {
          if (navigator.share) {
            navigator.share({ title, url: window.location.href }).catch(() => {})
          } else {
            navigator.clipboard.writeText(window.location.href)
          }
        }
      }}
    >
      <Share2 className="w-4 h-4" />
      Share
    </Button>
  )
}
