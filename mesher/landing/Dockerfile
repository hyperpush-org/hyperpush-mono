FROM node:20-slim AS deps
WORKDIR /app
# Install Python & build tools for native addons (better-sqlite3)
RUN apt-get update && apt-get install -y --no-install-recommends \
      python3 make g++ \
    && rm -rf /var/lib/apt/lists/*
COPY package.json package-lock.json ./
RUN npm ci

FROM node:20-slim AS builder
WORKDIR /app
ARG NEXT_PUBLIC_SITE_URL=https://hyperpush.dev
ARG NEXT_PUBLIC_DISCORD_URL=https://discord.gg/6SRhbZw7ZG
ARG NEXT_PUBLIC_WAITLIST_EMAIL=yurlovandrew@gmail.com
ARG NEXT_PUBLIC_FORMSPREE_ID=https://formspree.io/f/mreoyono
ENV NEXT_TELEMETRY_DISABLED=1
ENV NEXT_PUBLIC_SITE_URL=${NEXT_PUBLIC_SITE_URL}
ENV NEXT_PUBLIC_DISCORD_URL=${NEXT_PUBLIC_DISCORD_URL}
ENV NEXT_PUBLIC_WAITLIST_EMAIL=${NEXT_PUBLIC_WAITLIST_EMAIL}
ENV NEXT_PUBLIC_FORMSPREE_ID=${NEXT_PUBLIC_FORMSPREE_ID}
COPY --from=deps /app/node_modules ./node_modules
COPY . .
RUN npm run build

FROM node:20-slim AS runner
WORKDIR /app
ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1
ENV HOSTNAME=0.0.0.0
ENV PORT=3000
# Default DB path — override with DATABASE_PATH env var
ENV DATABASE_PATH=/data/blog.db

RUN groupadd --system --gid 1001 nodejs \
  && useradd --system --uid 1001 --gid 1001 nextjs

# Ensure /data volume mountpoint is owned by app user
RUN mkdir -p /data && chown nextjs:nodejs /data

COPY --from=builder --chown=nextjs:nodejs /app/public ./public
COPY --from=builder --chown=nextjs:nodejs /app/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/.next/static ./.next/static

# Copy better-sqlite3 native addon into standalone's node_modules
# Next.js standalone does not auto-copy .node files for serverExternalPackages
RUN mkdir -p ./node_modules/better-sqlite3
COPY --from=builder --chown=nextjs:nodejs /app/node_modules/better-sqlite3 ./node_modules/better-sqlite3
COPY --from=builder --chown=nextjs:nodejs /app/node_modules/bindings ./node_modules/bindings
COPY --from=builder --chown=nextjs:nodejs /app/node_modules/file-uri-to-path ./node_modules/file-uri-to-path

USER nextjs
EXPOSE 3000
CMD ["node", "server.js"]
