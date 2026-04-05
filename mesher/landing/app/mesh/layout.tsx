import type { Metadata } from "next"

export const metadata: Metadata = {
  title: "Built on Mesh — The Language Behind hyperpush",
  description:
    "Mesh is the programming language hyperpush was built in. Learn what it is, why we created it, and why it makes hyperpush unlike anything else in error tracking.",
  openGraph: {
    title: "Built on Mesh — The Language Behind hyperpush",
    description:
      "Mesh is the programming language hyperpush was built in. Learn what it is, why we created it, and why it makes hyperpush unlike anything else in error tracking.",
  },
}

export default function MeshLayout({ children }: { children: React.ReactNode }) {
  return children
}
