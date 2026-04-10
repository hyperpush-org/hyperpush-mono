import type { Metadata } from 'next'
import { deckData } from '@/lib/pitch/deck-data'
import { buildSocialMetadata } from '@/lib/social-metadata'

const socialMetadata = buildSocialMetadata({
  title: deckData.routeTitle,
  description: deckData.routeDescription,
  canonicalPath: '/pitch',
})

export const metadata: Metadata = {
  title: deckData.routeTitle,
  description: deckData.routeDescription,
  ...socialMetadata,
}

export default function PitchLayout({ children }: { children: React.ReactNode }) {
  return children
}
