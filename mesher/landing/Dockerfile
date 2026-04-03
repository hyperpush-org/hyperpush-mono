FROM node:20-slim AS deps
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci

FROM node:20-slim AS builder
WORKDIR /app
ARG NEXT_PUBLIC_SITE_URL=https://hyperpush.dev
ARG NEXT_PUBLIC_DISCORD_URL=https://discord.gg/hyperpush
ARG NEXT_PUBLIC_WAITLIST_EMAIL=waitlist@hyperpush.dev
ARG NEXT_PUBLIC_FORMSPREE_ID=
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
RUN groupadd --system --gid 1001 nodejs \
  && useradd --system --uid 1001 --gid 1001 nextjs
COPY --from=builder --chown=nextjs:nodejs /app/public ./public
COPY --from=builder --chown=nextjs:nodejs /app/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/.next/static ./.next/static
USER nextjs
EXPOSE 3000
CMD ["node", "server.js"]
