import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Plain HTML/CSS/JS in out/, so any static host (Cloudflare Pages, Vercel, Netlify) can serve it.
  output: "export",
  // /privacy -> out/privacy/index.html, which every static server resolves without rewrites.
  trailingSlash: true,
  // Images are pre-sized WebP from scripts/prepare-images.mjs; there is no server to optimise them.
  images: { unoptimized: true },
  turbopack: {
    rules: {
      "*.css": {
        loaders: ["@tailwindcss/turbopack"],
        as: "*.css",
      },
    },
  },
};

export default nextConfig;
