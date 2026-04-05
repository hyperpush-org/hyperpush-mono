import type { Metadata, Viewport } from 'next'
import { Geist, Geist_Mono } from 'next/font/google'
import { Analytics } from '@vercel/analytics/next'
import { X_HANDLE } from '@/lib/external-links'
import './globals.css'

const _geist = Geist({ subsets: ['latin'] })
const _geistMono = Geist_Mono({ subsets: ['latin'] })

const siteUrl = process.env.NEXT_PUBLIC_SITE_URL ?? 'https://hyperpush.dev'

const title = 'hyperpush — Open Source Error Tracking with Token Rewards'
const description =
  'Open-source error tracking with built-in token economics for Solana and beyond. Your project gets funded, contributors get paid, software gets healthier.'

export const metadata: Metadata = {
  metadataBase: new URL(siteUrl),
  title: {
    default: title,
    template: '%s | hyperpush',
  },
  description,
  keywords: [
    'error tracking',
    'open source',
    'solana',
    'bug bounties',
    'sentry alternative',
    'developer tools',
    'token economics',
    'bug board',
    'javascript error tracking',
    'rust error tracking',
  ],
  authors: [{ name: 'hyperpush', url: siteUrl }],
  creator: 'hyperpush',
  publisher: 'hyperpush',
  formatDetection: { email: false, address: false, telephone: false },
  openGraph: {
    type: 'website',
    url: siteUrl,
    title,
    description,
    siteName: 'hyperpush',
    images: [
      {
        url: '/x-banner.png',
        width: 3000,
        height: 1000,
        alt: 'hyperpush — Open Source Error Tracking with Token Rewards',
      },
    ],
    locale: 'en_US',
  },
  twitter: {
    card: 'summary_large_image',
    title,
    description,
    images: ['/x-banner.png'],
    site: X_HANDLE,
    creator: X_HANDLE,
  },
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
      'max-video-preview': -1,
      'max-image-preview': 'large',
      'max-snippet': -1,
    },
  },
  icons: {
    icon: [
      { url: '/favicon.svg', type: 'image/svg+xml' },
      { url: '/favicon.ico', sizes: '16x16 32x32 48x48', type: 'image/x-icon' },
      { url: '/favicon-16x16.png', sizes: '16x16', type: 'image/png' },
      { url: '/favicon-32x32.png', sizes: '32x32', type: 'image/png' },
      { url: '/favicon-192x192.png', sizes: '192x192', type: 'image/png' },
    ],
    apple: [
      { url: '/apple-touch-icon.png', sizes: '180x180', type: 'image/png' },
    ],
  },
  alternates: {
    canonical: siteUrl,
  },
}

export const viewport: Viewport = {
  width: 'device-width',
  initialScale: 1,
  maximumScale: 5,
  themeColor: [
    { media: '(prefers-color-scheme: dark)', color: '#141414' },
    { media: '(prefers-color-scheme: light)', color: '#141414' },
  ],
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html lang="en" className="scroll-smooth">
      <body className="font-sans antialiased overflow-x-hidden">
        {children}
        <Analytics />
      </body>
    </html>
  )
}
