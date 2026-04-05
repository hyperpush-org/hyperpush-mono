import Database from 'better-sqlite3'
import path from 'path'
import fs from 'fs'

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------
function resolveDbPath(): string {
  if (process.env.DATABASE_PATH) return process.env.DATABASE_PATH
  if (process.env.NODE_ENV === 'production') return '/data/blog.db'
  return path.join(process.cwd(), 'data', 'blog.db')
}

function resolveDataDir(): string {
  const dbPath = resolveDbPath()
  return path.dirname(dbPath)
}

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------
const SCHEMA = `
  CREATE TABLE IF NOT EXISTS posts (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    slug            TEXT    NOT NULL UNIQUE,
    title           TEXT    NOT NULL,
    excerpt         TEXT    NOT NULL DEFAULT '',
    content         TEXT    NOT NULL DEFAULT '',
    author          TEXT    NOT NULL DEFAULT 'Core Team',
    category        TEXT    NOT NULL DEFAULT 'Engineering',
    status          TEXT    NOT NULL DEFAULT 'draft'
                    CHECK (status IN ('draft','published')),
    cover_image     TEXT,
    read_time_minutes INTEGER NOT NULL DEFAULT 5,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    published_at    INTEGER
  );

  CREATE INDEX IF NOT EXISTS idx_posts_slug         ON posts(slug);
  CREATE INDEX IF NOT EXISTS idx_posts_status       ON posts(status);
  CREATE INDEX IF NOT EXISTS idx_posts_published_at ON posts(published_at DESC);
`

// ---------------------------------------------------------------------------
// Seed data (the existing hardcoded posts)
// ---------------------------------------------------------------------------
const SEED_POSTS = [
  {
    slug: 'introducing-mesher',
    title: "Introducing hyperpush: Open-Source Error Tracking That Funds Itself",
    excerpt: "We're building the error tracking platform we always wanted — open source, self-hostable, with built-in economics that fund projects and pay the developers who fix bugs.",
    author: 'Core Team',
    category: 'Announcement',
    status: 'published' as const,
    cover_image: null,
    read_time_minutes: 8,
    content: `<h2>The Problem</h2><p>Open-source software powers the internet, but maintaining it is thankless. Projects accumulate bugs faster than maintainers can fix them. Contributors want to help but have no incentive beyond goodwill. Error tracking tools that could surface and prioritize these bugs cost money that projects don't have.</p><p>The result is a vicious cycle: bugs pile up, users lose trust, contributors move on, and projects stagnate.</p><h2>Our Approach</h2><p>hyperpush breaks this cycle by aligning incentives. Every project gets full error tracking for free. Projects that opt in can launch a token that funds their treasury through trading activity. That treasury funds bounties on bugs. Developers fix bugs and earn bounties. The project gets healthier, the developer gets paid, and the token gains value as the software improves.</p><h3>What You Get on Day One</h3><ul><li>Full error tracking — stack traces, session replays, performance monitoring, and alerts</li><li>Solana program error tracking — transaction failures, CPI errors, RPC timeouts</li><li>Public bug board — opt-in dashboard showing live errors with bounties</li><li>Token economics — launch a project token that funds development</li><li>GitHub PR verification — automated bounty distribution on merged fixes</li></ul><h2>Built on Mesh</h2><p>hyperpush runs on Mesh, a systems language with Elixir's fault-tolerant actor model and the raw speed of compiled code. Every error event is handled by its own lightweight process — millions of concurrent events, sub-millisecond spawn times, and supervision trees that self-heal on failure.</p><h2>What's Next</h2><p>We're opening the waitlist today. The first 10 qualifying open-source projects get the full Pro tier free for 6 months. If you maintain an open-source project on Solana — or anywhere else — we'd love to have you.</p>`,
    published_at: new Date('2026-03-24').getTime(),
    created_at: new Date('2026-03-24').getTime(),
    updated_at: new Date('2026-03-24').getTime(),
  },
  {
    slug: 'why-token-economics',
    title: "Why We Chose Token Economics Over SaaS Pricing",
    excerpt: "Most open-source projects die from lack of funding. We explored every model — sponsorships, SaaS, foundations — before landing on token economics. Here's why.",
    author: 'Core Team',
    category: 'Deep Dive',
    status: 'published' as const,
    cover_image: null,
    read_time_minutes: 12,
    content: `<p>Every open-source project faces the same question: how do you sustain development without compromising the open-source promise? We spent months evaluating every funding model before choosing token economics. This post explains our reasoning.</p><h2>The Funding Graveyard</h2><p>GitHub Sponsors averages $50/month for most projects. Foundations require years of governance overhead. Dual licensing creates perverse incentives. Open-core models inevitably move the best features behind paywalls. We needed something that scales with usage and aligns contributor incentives.</p><h2>Why Tokens Work Here</h2><p>Token economics aren't right for every project. But error tracking has a unique property: the more bugs that get fixed, the more valuable the tool becomes. A project token that gains value as software quality improves creates a flywheel where everyone — projects, developers, and users — benefits from the same outcome.</p><h3>The Mechanics</h3><ul><li>Each project can launch a token at no cost through hyperpush</li><li>A portion of trading activity flows into the project treasury</li><li>The treasury funds bounties on bugs surfaced by hyperpush</li><li>Developers fix bugs, submit PRs, and earn bounties automatically</li><li>The project gets healthier, increasing the token's utility and value</li></ul><h2>Addressing the Skepticism</h2><p>We know "crypto" raises eyebrows. But we're not building a speculative instrument — we're building a funding mechanism. The token's value is directly tied to software quality: more bugs fixed → healthier project → more adoption → more treasury funding. No speculation required.</p>`,
    published_at: new Date('2026-03-20').getTime(),
    created_at: new Date('2026-03-20').getTime(),
    updated_at: new Date('2026-03-20').getTime(),
  },
  {
    slug: 'solana-error-tracking',
    title: "First-Class Solana Error Tracking: What It Actually Means",
    excerpt: "Every error tracker can capture console.error. But tracking Solana program failures — CPI errors, transaction simulation failures, RPC timeouts — requires purpose-built tooling.",
    author: 'Core Team',
    category: 'Technical',
    status: 'published' as const,
    cover_image: null,
    read_time_minutes: 10,
    content: `<p>If you've built on Solana, you know the pain. A user reports that a transaction failed, and all you have is a vague error message and a transaction signature. You dig through explorer, decode the logs, realize it was a CPI failure three calls deep, and spend an hour recreating the state to understand what went wrong.</p><h2>What Existing Tools Miss</h2><p>General-purpose error trackers treat blockchain interactions like any other HTTP call. They'll capture the fetch error when your RPC call fails, but they won't tell you why the transaction failed at the program level. The error context that actually matters — instruction data, account states, CPI call chains — is invisible.</p><h2>hyperpush's Approach</h2><p>We instrument at the SDK level, capturing the full transaction context before it's submitted. When a transaction fails — whether during simulation or on-chain — we capture the program logs, decode the error, trace the CPI chain, and surface the root cause in a human-readable format.</p><h3>What Gets Captured</h3><ul><li>Transaction simulation failures with decoded program errors</li><li>On-chain transaction failures with full instruction trace</li><li>CPI call chain visualization showing exactly where the error originated</li><li>Account state snapshots at the time of failure</li><li>RPC timeout and rate-limit tracking across providers</li><li>Priority fee spike correlation with error rates</li></ul><p>This isn't a wrapper around explorer APIs. It's purpose-built instrumentation that gives Solana developers the same debugging experience that web developers take for granted.</p>`,
    published_at: new Date('2026-03-16').getTime(),
    created_at: new Date('2026-03-16').getTime(),
    updated_at: new Date('2026-03-16').getTime(),
  },
]

// ---------------------------------------------------------------------------
// Singleton
// ---------------------------------------------------------------------------
declare global {
  // eslint-disable-next-line no-var
  var __blogDb: Database.Database | undefined
}

function initDb(): Database.Database {
  const dataDir = resolveDataDir()
  const dbPath = resolveDbPath()

  try {
    fs.mkdirSync(dataDir, { recursive: true })
  } catch {
    // ignore — may already exist
  }

  const db = new Database(dbPath)
  db.pragma('journal_mode = WAL')
  db.pragma('synchronous = NORMAL')
  db.pragma('foreign_keys = ON')
  db.exec(SCHEMA)

  // Seed if empty
  const count = (db.prepare('SELECT COUNT(*) as n FROM posts').get() as { n: number }).n
  if (count === 0) {
    const insert = db.prepare(`
      INSERT INTO posts
        (slug, title, excerpt, content, author, category, status,
         cover_image, read_time_minutes, created_at, updated_at, published_at)
      VALUES
        (@slug, @title, @excerpt, @content, @author, @category, @status,
         @cover_image, @read_time_minutes, @created_at, @updated_at, @published_at)
    `)
    const seedAll = db.transaction(() => {
      for (const p of SEED_POSTS) insert.run(p)
    })
    seedAll()
  }

  return db
}

export function getDb(): Database.Database {
  if (!global.__blogDb) {
    global.__blogDb = initDb()
  }
  return global.__blogDb
}
