import type { Metadata, Viewport } from "next";
import { Inter, JetBrains_Mono } from "next/font/google";

import { DownloadBar } from "@/components/download-bar";
import { SiteFooter } from "@/components/site-footer";
import { PITCH, SITE_URL } from "@/lib/site";
import "./globals.css";

const inter = Inter({ variable: "--font-inter", subsets: ["latin"], display: "swap" });
const jetbrains = JetBrains_Mono({
  variable: "--font-jetbrains",
  subsets: ["latin"],
  weight: ["400", "500"],
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
  themeColor: "#1a1816",
  colorScheme: "dark",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en" className={`${inter.variable} ${jetbrains.variable} antialiased`}>
      <body className="min-h-dvh">
        <a
          href="#main"
          className="sr-only z-50 rounded-full bg-accent px-4 py-2 text-sm font-semibold text-on-accent focus:not-sr-only focus:fixed focus:top-2 focus:left-2"
        >
          Skip to content
        </a>
        <DownloadBar />
        <main id="main">{children}</main>
        <SiteFooter />
      </body>
    </html>
  );
}
