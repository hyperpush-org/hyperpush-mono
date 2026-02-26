---
phase: quick-6
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - .planning/quick/6-write-an-article-about-the-benchmarks-in/ARTICLE.md
autonomous: true
requirements: [BENCH-ARTICLE-01]

must_haves:
  truths:
    - "Article explains what was benchmarked and why"
    - "Article states Mesh beats Elixir in throughput (~20K vs ~12K req/s)"
    - "Article honestly acknowledges Mesh is slower than Go and Rust"
    - "Article explains the actor model overhead trade-off"
    - "Article includes the actual numbers from RESULTS.md"
    - "Article follows the terse, first-person tone of the previous HN post"
  artifacts:
    - path: ".planning/quick/6-write-an-article-about-the-benchmarks-in/ARTICLE.md"
      provides: "Published benchmark article in Markdown"
  key_links:
    - from: "RESULTS.md numbers"
      to: "Article claims"
      via: "Inline text and table"
      pattern: "19,718|20,483|11,842|26,278|27,133"
---

<objective>
Write a Markdown article about the Mesh HTTP benchmarks that can be posted to HN or a blog.

Purpose: Share honest benchmark results that position Mesh accurately — faster than Elixir despite carrying actor model overhead, slower than Go/Rust which have no such overhead, native-compiled with a tiny memory footprint.

Output: ARTICLE.md ready to post, ~400-600 words, same sparse direct voice as the previous HN article.
</objective>

<execution_context>
@/Users/sn0w/.claude/get-shit-done/workflows/execute-plan.md
</execution_context>

<context>
@.planning/quick/2-mesh-story-article/ARTICLE_HN.txt
@benchmarks/RESULTS.md
@benchmarks/METHODOLOGY.md
@benchmarks/mesh/main.mpl
</context>

<tasks>

<task type="auto">
  <name>Task 1: Write benchmark article</name>
  <files>.planning/quick/6-write-an-article-about-the-benchmarks-in/ARTICLE.md</files>
  <action>
Write a Markdown article to the output file. Use these exact numbers from RESULTS.md:

  /text:  Mesh 19,718 req/s | Go 26,278 | Rust 27,133 | Elixir 11,842
  /json:  Mesh 20,483 req/s | Go 26,175 | Rust 28,563 | Elixir 11,481
  Latency (Go, p50/p99): /text 3.1ms / 14.1ms, /json 3.0ms / 14.1ms
  Latency (Rust, p50/p99): /text 2.8ms / 14.5ms, /json 2.9ms / 13.7ms
  Latency (Elixir, p50/p99): /text 7.8ms / 19.7ms, /json 7.7ms / 19.3ms
  Mesh p50/p99: not recorded (latency parser fix arrived after the run)

Hardware: Fly.io performance-2x (2 dedicated vCPU, 4 GB RAM), two VMs in ord (Chicago), intra-datacenter WireGuard network. Load generator on a separate VM. 100 concurrent connections, HTTP/1.1, hey load tester. 30s warmup + 5 timed runs of 30s each, Run 1 excluded, Runs 2-5 averaged.

Tone and style: Match the previous ARTICLE_HN.txt — terse, direct, first person. Short sections with bold headers. No marketing. Include a code snippet of the Mesh server (9 lines from benchmarks/mesh/main.mpl). Acknowledge limitations honestly (all four servers co-located on the same 2-vCPU VM, Mesh p50/p99 missing).

Structure the article roughly as:

1. Opening hook: what this is (Mesh HTTP bench vs Go/Rust/Elixir, real hardware)
2. THE NUMBERS — results table both endpoints
3. THE MESH SERVER — show the 9-line Mesh code
4. WHAT THE NUMBERS MEAN — Mesh beats Elixir (~66% faster) despite actor model overhead; Go and Rust have no actor overhead at all, so their ~35% lead over Mesh is the cost of having a supervised concurrent runtime baked in; native compile + LLVM gives the throughput floor
5. THE ACTOR OVERHEAD — explain what the overhead IS: every HTTP request goes through the mesh-rt actor scheduler (process spawning, mailbox dispatch, supervision tree); this is structural, not accidental; the same machinery that gives you fault isolation and location-transparent PIDs has a cost
6. MEMORY — Mesh 4.9 MB startup RSS vs Elixir 1.6 MB vs Rust 3.4 MB vs Go 1.5 MB; note Mesh is higher than Elixir at idle but produces ~66% more throughput
7. METHODOLOGY in brief — two Fly.io VMs, hey, 100c, 30s warmup, runs 2-5 averaged — link to METHODOLOGY.md and RESULTS.md
8. Links: meshlang.dev, repo

Do NOT be defensive about being slower than Go/Rust. The framing is: Mesh is a language with an actor runtime built in; Go and Rust are doing raw thread-pool HTTP with no actor machinery. The correct comparison for Elixir vs Mesh is actor-model language vs actor-model language, and Mesh wins there by ~66%. The Go/Rust gap is the honest cost of the abstraction.
  </action>
  <verify>
    <automated>grep -c "19,718\|19718" /Users/sn0w/Documents/dev/snow/.planning/quick/6-write-an-article-about-the-benchmarks-in/ARTICLE.md && grep -c "11,842\|11842" /Users/sn0w/Documents/dev/snow/.planning/quick/6-write-an-article-about-the-benchmarks-in/ARTICLE.md</automated>
    <manual>Read the article. Check: numbers match RESULTS.md, actor overhead is explained honestly, tone matches the HN article, no marketing fluff.</manual>
  </verify>
  <done>ARTICLE.md exists, contains the real benchmark numbers, explains the actor overhead trade-off, beats-Elixir framing is clear, article is 400-600 words.</done>
</task>

</tasks>

<verification>
- ARTICLE.md contains exact req/s numbers from RESULTS.md
- Mesh vs Elixir comparison is the headline performance claim
- Actor model overhead is explained structurally (not apologetically)
- Go/Rust lead over Mesh is framed as the cost of the actor runtime, not a failure
- Code snippet from benchmarks/mesh/main.mpl is present
- Links to meshlang.dev and benchmark docs included
</verification>

<success_criteria>
Article is honest, uses real numbers, explains the performance story accurately (Mesh > Elixir, actor overhead explains gap to Go/Rust), and matches the terse direct tone of the previous article.
</success_criteria>

<output>
Article written to: .planning/quick/6-write-an-article-about-the-benchmarks-in/ARTICLE.md
No SUMMARY.md required for quick tasks.
</output>
