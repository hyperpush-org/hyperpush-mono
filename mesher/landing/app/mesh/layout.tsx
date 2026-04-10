import type { Metadata } from 'next'
import { buildSocialMetadata } from '@/lib/social-metadata'

const title = 'Built on Mesh — The Language Behind hyperpush'
const description =
  'Mesh is the programming language hyperpush was built in. Learn what it is, why we created it, and why it makes hyperpush unlike anything else in error tracking.'
const socialMetadata = buildSocialMetadata({
  title,
  description,
  canonicalPath: '/mesh',
})

export const metadata: Metadata = {
  title,
  description,
  ...socialMetadata,
}

export default function MeshLayout({ children }: { children: React.ReactNode }) {
  return children
}
