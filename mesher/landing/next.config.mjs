/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'standalone',
  typescript: {
    ignoreBuildErrors: true,
  },
  images: {
    unoptimized: true,
  },
  // Prevent bundling of native Node.js modules — let them require() at runtime
  serverExternalPackages: ['better-sqlite3'],
}

export default nextConfig
