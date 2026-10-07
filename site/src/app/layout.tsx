import type { Metadata, Viewport } from "next";
import { Archivo } from "next/font/google";

import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";
import { PITCH, SITE_URL } from "@/lib/site";
import "./globals.css";

const archivo = Archivo({
  variable: "--font-archivo",
  subsets: ["latin"],
  axes: ["wdth"],
  display: "swap",
});

const description =
  "Snag is a free, open-source download manager for Windows. It catches any video like IDM, downloads files over 8 connections at once, and keeps everything on your PC.";

export const metadata: Metadata = {
  metadataBase: new URL(SITE_URL),
  title: {
    default: "Snag — the fast, free download manager for Windows",
    template: "%s · Snag",
  },
  description,
  applicationName: "Snag",
  authors: [{ name: "Kuldeep Saini", url: "https://github.com/kuldeepsaini23" }],
  keywords: [
    "download manager",
    "Windows",
    "IDM alternative",
    "video downloader",
    "yt-dlp",
    "open source",
    "browser extension",
  ],
  alternates: { canonical: "/" },
  openGraph: {
    type: "website",
    url: "/",
    siteName: "Snag",
    title: "Snag — the fast, free download manager for Windows",
    description: PITCH,
    images: [{ url: "/og.png", width: 1200, height: 630, alt: "Snag downloading files, next to its name and pitch" }],
  },
  twitter: {
    card: "summary_large_image",
    title: "Snag — the fast, free download manager for Windows",
    description: PITCH,
    images: ["/og.png"],
  },
};

export const viewport: Viewport = {
  themeColor: "#1c1a18",
  colorScheme: "dark",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en" className={`${archivo.variable} antialiased`}>
      <body className="flex min-h-dvh flex-col">
        <a
          href="#main"
          className="sr-only z-50 rounded-full bg-accent px-4 py-2 font-semibold text-on-accent focus:not-sr-only focus:fixed focus:top-3 focus:left-3"
        >
          Skip to content
        </a>
        <SiteHeader />
        <main id="main" className="flex-1">
          {children}
        </main>
        <SiteFooter />
      </body>
    </html>
  );
}
