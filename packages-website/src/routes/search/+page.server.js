export async function load({ fetch, url }) {
  const q = url.searchParams.get('q') || '';
  if (!q.trim()) return { packages: [], query: q };
  try {
    const res = await fetch(`https://api.packages.meshlang.dev/api/v1/packages?search=${encodeURIComponent(q)}`);
    if (!res.ok) return { packages: [], query: q, error: 'Registry unavailable' };
    const packages = await res.json();
    return { packages, query: q };
  } catch {
    return { packages: [], query: q, error: 'Search failed' };
  }
}
