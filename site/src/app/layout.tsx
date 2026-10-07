import type { Metadata, Viewport } from "next";
import { Archivo, Instrument_Serif, Inter, JetBrains_Mono } from "next/font/google";

import { RevealObserver } from "@/components/reveal-observer";
import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";
import { PITCH, SITE_URL } from "@/lib/site";
import "./globals.css";

const inter = Inter({ variable: "--font-inter", subsets: ["latin"], display: "swap" });
const archivo = Archivo({
  variable: "--font-archivo",
  subsets: ["latin"],
  axes: ["wdth"],
  display: "swap",
});
const instrument = Instrument_Serif({
  variable: "--font-instrument",
  subsets: ["latin"],
  weight: "400",
  style: "italic",
  display: "swap",
});
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
  themeColor: "#111216",
  colorScheme: "dark",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html
      suppressHydrationWarning
      lang="en"
      className={`${inter.variable} ${archivo.variable} ${instrument.variable} ${jetbrains.variable} antialiased`}
    >
      <head>
        {/* Scroll reveals hide content only when scripts run, so the page is whole without them. */}
        <script dangerouslySetInnerHTML={{ __html: "document.documentElement.classList.add('js')" }} />
      </head>
      <body className="min-h-dvh">
        <a
          href="#main"
          className="label sr-only z-50 bg-accent px-4 py-3 text-on-accent focus:not-sr-only focus:fixed focus:top-2 focus:left-2"
        >
          Skip to content
        </a>
        <SiteHeader />
        {/* The column: hairlines down both sides, centred in the space left of the desktop rail. */}
        <div className="lg:pr-(--rail-w)">
          <div className="mx-auto max-w-[66rem] border-line sm:border-x">
            <main id="main">{children}</main>
            <SiteFooter />
          </div>
        </div>
        <RevealObserver />
      </body>
    </html>
  );
}
