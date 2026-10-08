import type { Metadata, Viewport } from "next";
import { Fraunces, Inter, JetBrains_Mono } from "next/font/google";

import { DownloadBar } from "@/components/download-bar";
import { Reveal } from "@/components/reveal";
import { SiteFooter } from "@/components/site-footer";
import { DOWNLOAD_URL, PITCH, RELEASES_URL, REPO_URL, SITE_URL, VERSION } from "@/lib/site";
import "./globals.css";

const inter = Inter({ variable: "--font-inter", subsets: ["latin"], display: "swap" });
// The catalogue serif, italic only: the one word in each headline, the numerals and the captions.
// At its default optical size it has the sturdier cut of an old type specimen, and one file
// keeps the headline from waiting on fonts.
const fraunces = Fraunces({
  variable: "--font-fraunces",
  subsets: ["latin"],
  style: ["italic"],
  display: "swap",
});
const jetbrains = JetBrains_Mono({
  variable: "--font-jetbrains",
  subsets: ["latin"],
  weight: ["400", "500"],
  display: "swap",
  // Labels only; the headline must not wait on it.
  preload: false,
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
  themeColor: "#0b0b0c",
  colorScheme: "dark",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en" className={`${inter.variable} ${fraunces.variable} ${jetbrains.variable} antialiased`}>
      {/* Browser extensions often add attributes to <body> before React loads; that is not a real mismatch. */}
      <body className="min-h-dvh" suppressHydrationWarning>
        <a
          href="#main"
          className="sr-only z-50 bg-accent px-4 py-2 text-sm font-semibold text-on-accent focus:not-sr-only focus:fixed focus:top-2 focus:left-2"
        >
          Skip to content
        </a>
        <DownloadBar downloadUrl={DOWNLOAD_URL} releasesUrl={RELEASES_URL} repoUrl={REPO_URL} version={VERSION} />
        <main id="main">{children}</main>
        <SiteFooter />
        <Reveal />
      </body>
    </html>
  );
}
