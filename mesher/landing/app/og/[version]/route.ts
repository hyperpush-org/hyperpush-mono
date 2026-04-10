import { readFile } from 'node:fs/promises'
import { join } from 'node:path'

export const runtime = 'nodejs'

export async function GET(
  _request: Request,
  { params }: { params: Promise<{ version: string }> }
) {
  const { version } = await params
  const image = await readFile(join(process.cwd(), 'public', 'og-image.png'))
  const isVersioned = version !== 'current'

  return new Response(image, {
    headers: {
      'Content-Type': 'image/png',
      'Content-Length': image.byteLength.toString(),
      'Cache-Control': isVersioned
        ? 'public, max-age=31536000, immutable'
        : 'public, max-age=0, s-maxage=3600, stale-while-revalidate=86400',
      'X-Robots-Tag': 'noindex',
    },
  })
}
