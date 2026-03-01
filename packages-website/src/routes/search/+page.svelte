<script>
  import { Search } from 'lucide-svelte';
  export let data;
</script>

<section class="border-b border-border bg-background py-10">
  <div class="mx-auto max-w-6xl px-4">
    <div class="text-xs font-mono uppercase tracking-widest text-muted-foreground mb-1">Search</div>
    <h1 class="text-2xl font-bold tracking-tight text-foreground">
      {#if data.query}
        Results for "{data.query}"
      {:else}
        Search packages
      {/if}
    </h1>
    {#if !data.error && data.query}
      <p class="mt-1 text-sm text-muted-foreground">
        {data.packages.length} result{data.packages.length === 1 ? '' : 's'}
      </p>
    {/if}
  </div>
</section>

<section class="py-10">
  <div class="mx-auto max-w-6xl px-4">
    {#if data.error}
      <div class="rounded-xl border border-border bg-card p-8 text-center">
        <p class="text-muted-foreground">{data.error}</p>
      </div>
    {:else if !data.query}
      <div class="rounded-xl border border-border bg-card p-8 text-center">
        <Search class="mx-auto size-8 text-muted-foreground mb-3" />
        <p class="text-muted-foreground">Enter a query to search packages.</p>
      </div>
    {:else if data.packages.length === 0}
      <div class="rounded-xl border border-border bg-card p-8 text-center">
        <p class="text-muted-foreground">No packages found for "{data.query}".</p>
        <a href="/" class="mt-4 inline-block text-sm text-foreground underline underline-offset-4 hover:text-muted-foreground">
          Browse all packages
        </a>
      </div>
    {:else}
      <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {#each data.packages as pkg}
          <a
            href="/packages/{pkg.name}"
            class="block rounded-xl border border-foreground/10 bg-card p-6 transition-all duration-300 hover:-translate-y-0.5 hover:border-foreground/30 hover:shadow-lg no-underline"
          >
            <div class="flex items-start justify-between gap-2">
              <span class="text-base font-bold text-foreground leading-tight">{pkg.name}</span>
              <span class="shrink-0 rounded-md bg-muted px-2 py-0.5 font-mono text-xs text-muted-foreground whitespace-nowrap">
                v{pkg.version}
              </span>
            </div>
            <p class="mt-2 text-sm leading-relaxed text-muted-foreground line-clamp-2">
              {pkg.description || 'No description provided.'}
            </p>
          </a>
        {/each}
      </div>
    {/if}
  </div>
</section>
