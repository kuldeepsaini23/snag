import type { Metadata } from "next";

import { ISSUES_URL, REPO_URL } from "@/lib/site";

export const metadata: Metadata = {
  title: "Privacy policy",
  description:
    "What the Snag app and the Snag browser extension read, where it goes (only to your own PC), and what they never do.",
  alternates: { canonical: "/privacy/" },
  // Metadata merges shallowly, so the card is repeated here rather than inherited.
  openGraph: {
    type: "website",
    url: "/privacy/",
    siteName: "Snag",
    title: "Snag privacy policy",
    description: "No account, no analytics, no server. What the Snag app and extension read, and where it goes.",
    images: [{ url: "/og.png", width: 1200, height: 630, alt: "Snag, the download manager for Windows" }],
  },
};

function Section({ id, title, children }: { id: string; title: string; children: React.ReactNode }) {
  return (
    <section id={id} className="mt-4 rounded-2xl bg-panel p-5 shadow-[0_0_0_1px_var(--line)] sm:p-8">
      <h2 className="text-2xl font-bold tracking-tight">{title}</h2>
      <div className="mt-4 grid max-w-[68ch] gap-4 leading-relaxed text-muted [&_strong]:font-semibold [&_strong]:text-text">
        {children}
      </div>
    </section>
  );
}

function List({ children }: { children: React.ReactNode }) {
  return <ul className="grid list-disc gap-2 pl-5 marker:text-accent">{children}</ul>;
}

function Code({ children }: { children: React.ReactNode }) {
  return <code className="rounded bg-raised px-1.5 py-0.5 font-mono text-[0.85em] text-text">{children}</code>;
}

const link = "text-accent underline decoration-accent/40 underline-offset-4 hover:decoration-accent";

export default function PrivacyPage() {
  return (
    <article className="mx-auto max-w-3xl px-4 pt-12 pb-24 sm:px-6 md:pt-20">
      <header>
        <p className="font-mono text-xs text-faint">Effective 7 October 2026</p>
        <h1 className="mt-3 text-5xl font-bold tracking-tight md:text-6xl">Privacy policy</h1>
        <p className="mt-5 max-w-[60ch] text-lg leading-relaxed text-muted">
          For the Snag desktop app and the Snag browser extension for Chrome, Edge and Firefox.
        </p>
      </header>

      <div className="mt-10 rounded-2xl bg-raised p-5 sm:p-8">
        <p className="flex items-center gap-2 font-mono text-xs text-accent">In short</p>
        <p className="mt-3 text-lg leading-relaxed">
          Snag has no account, no analytics, no ads and no server of its own. The extension sends what it reads only to
          the Snag app on the same computer. Nothing is sent to the developer or to any third party, and nothing is sold.
        </p>
      </div>

      <Section id="extension" title="The browser extension">
        <p>The extension exists to hand downloads to the Snag app on your computer. To do that, it reads:</p>
        <List>
          <li>
            the addresses (URLs), types and sizes of the media your open tabs load, so it can offer to download the video
            or song that is playing;
          </li>
          <li>the address of the page you are on;</li>
          <li>
            <strong>only when you send a download</strong>: the cookies of the site you are downloading from, and that
            page’s address as the referrer, because many sites refuse a download without them;
          </li>
          <li>
            <strong>only when you open Grab from page</strong>: the links, images and media on that page, so you can pick
            which ones to download.
          </li>
        </List>
        <p>
          All of this goes <strong>only to the Snag app on the same computer</strong>, at <Code>127.0.0.1</Code>, and only
          after you have allowed the connection once in Snag. It is never sent to the developer or to anyone else. The
          extension contains no analytics, no tracking and no ads.
        </p>
        <p>The extension keeps a few things in your browser’s own extension storage:</p>
        <List>
          <li>the pairing code that lets it talk to Snag;</li>
          <li>your choices: where the Download button sits, the sites you have hidden it on, and Snag’s accent colour;</li>
          <li>links you sent while Snag was closed, until Snag opens and takes them;</li>
          <li>the list of media found in each open tab, which is forgotten when you close the tab.</li>
        </List>
        <p>Removing the extension removes all of it.</p>
        <p>Why it asks for each browser permission:</p>
        <List>
          <li>
            <strong>Access to the sites you visit</strong> and <strong>web requests</strong>: to notice the media a page
            loads and show the Download button on it.
          </li>
          <li>
            <strong>Cookies</strong>: to pass the current site’s cookies to Snag when you send a download from it.
          </li>
          <li>
            <strong>Downloads</strong>: to hand downloads you start in the browser over to Snag.
          </li>
          <li>
            <strong>Scripting</strong>: to list a page’s links and images when you open Grab from page.
          </li>
          <li>
            <strong>Context menus</strong>, <strong>storage</strong> and <strong>alarms</strong>: for the right-click
            “Download with Snag” items, the settings above, and sending links that waited while Snag was closed.
          </li>
        </List>
      </Section>

      <Section id="app" title="The Snag app">
        <p>
          Snag keeps your download list, history and settings in <Code>%APPDATA%\Snag</Code> on your PC. Downloaded files
          go to the folders you choose. None of it is uploaded anywhere.
        </p>
        <p>Snag connects only to:</p>
        <List>
          <li>the sites you download from;</li>
          <li>
            <strong>GitHub</strong>, to fetch and update its helpers yt-dlp (for video sites) and gallery-dl (for image
            galleries);
          </li>
          <li>
            <strong>gyan.dev</strong>, once, to fetch ffmpeg the first time you need it for HD video or MP3;
          </li>
          <li>
            <strong>VirusTotal</strong>, only if you add your own API key, and then only the downloaded file’s SHA-256
            fingerprint is sent, never the file itself.
          </li>
        </List>
        <p>
          <strong>Clipboard watch</strong>, if you leave it on, looks at text you copy on your PC to spot links. That text
          stays on your PC.
        </p>
        <p>
          <strong>Phone sharing</strong> is off by default. When you turn it on, phones on your own local network can send
          links to Snag.
        </p>
        <p>
          <strong>Bug reports</strong> are a text file Snag writes on your PC. Nothing is sent automatically: you read it
          and choose whether to share it. It leaves out your pairing code, cookies, keys and personal folder names.
        </p>
      </Section>

      <Section id="never" title="What Snag never does">
        <List>
          <li>No accounts, sign-ins or email addresses.</li>
          <li>No analytics, telemetry, tracking or advertising.</li>
          <li>No selling or sharing of your data, because none of it reaches the developer.</li>
        </List>
        <p>
          Snag is open source, so you can check all of this in the{" "}
          <a href={REPO_URL} className={link}>
            source code
          </a>
          .
        </p>
      </Section>

      <Section id="website" title="This website">
        <p>
          snag.kuldeepsaini.dev is a static site with no cookies, analytics or trackers. The company that hosts it may keep
          standard server logs, such as IP addresses, for security.
        </p>
      </Section>

      <Section id="contact" title="Changes and contact">
        <p>
          If this policy changes, the new version will be posted here with a new effective date, and the change will be
          visible in the project’s history on GitHub.
        </p>
        <p>
          Questions or concerns? Open an issue at{" "}
          <a href={ISSUES_URL} className={link}>
            github.com/kuldeepsaini23/snag/issues
          </a>
          .
        </p>
      </Section>
    </article>
  );
}
