import type { Metadata } from 'next'
import { X_HANDLE } from '@/lib/external-links'

const DEFAULT_SITE_URL = 'https://hyperpush.dev'
const DEFAULT_SOCIAL_IMAGE_ALT =
  'hyperpush — Open Source Error Tracking with Token Rewards'
const DEFAULT_SOCIAL_IMAGE_SIZE = {
  width: 1200,
  height: 630,
} as const

export const siteUrl = process.env.NEXT_PUBLIC_SITE_URL ?? DEFAULT_SITE_URL

function normalizeOgImageVersion(value?: string) {
  const normalized = value
    ?.trim()
    .replace(/[^a-zA-Z0-9_-]+/g, '-')
    .replace(/^-+|-+$/g, '')

  return normalized ? normalized.slice(0, 32) : 'current'
}

export function resolveOgImageVersion() {
  return normalizeOgImageVersion(
    process.env.OG_IMAGE_VERSION ??
      process.env.VERCEL_GIT_COMMIT_SHA ??
      process.env.GIT_COMMIT_SHA ??
      process.env.VERCEL_DEPLOYMENT_ID
  )
}

export function getOgImageUrl() {
  return new URL(`/og/${resolveOgImageVersion()}`, siteUrl).toString()
}

export function getDefaultSocialImage() {
  return {
    url: getOgImageUrl(),
    ...DEFAULT_SOCIAL_IMAGE_SIZE,
    alt: DEFAULT_SOCIAL_IMAGE_ALT,
  }
}

type SocialMetadataOptions = {
  title: string
  description: string
  canonicalPath: string
}

export function buildSocialMetadata({
  title,
  description,
  canonicalPath,
}: SocialMetadataOptions): Pick<
  Metadata,
  'alternates' | 'openGraph' | 'twitter'
> {
  const canonicalUrl = new URL(canonicalPath, siteUrl).toString()
  const socialImage = getDefaultSocialImage()

  return {
    alternates: {
      canonical: canonicalUrl,
    },
    openGraph: {
      title,
      description,
      url: canonicalUrl,
      siteName: 'hyperpush',
      type: 'website',
      locale: 'en_US',
      images: [socialImage],
    },
    twitter: {
      card: 'summary_large_image',
      title,
      description,
      images: [socialImage.url],
      site: X_HANDLE,
      creator: X_HANDLE,
    },
  }
}
