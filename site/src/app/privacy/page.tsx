import type { Metadata } from "next";

import { ExternalLink } from "@/components/external-link";
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

const order = ["extension", "app", "never", "website", "contact"];

// Each section is a numbered article of the notice, ruled off from the next.
function Section({ id, title, children }: { id: string; title: string; children: React.ReactNode }) {
  return (
    <section
      id={id}
      className="grid gap-x-10 gap-y-4 border-t border-line py-12 md:grid-cols-[6rem_minmax(0,1fr)] md:py-16"
      data-reveal
    >
      <p className="font-display text-4xl leading-none font-light text-faint italic md:text-5xl">
        <span className="text-xl">§</span> {order.indexOf(id) + 1}
      </p>
      <div>
        <h2 className="headline text-3xl md:text-4xl">{title}</h2>
        <div className="mt-6 grid max-w-[68ch] gap-4 leading-relaxed text-muted [&_strong]:font-semibold [&_strong]:text-text">
          {children}
        </div>
      </div>
    </section>
  );
}

function List({ children }: { children: React.ReactNode }) {
  return <ul className="grid list-[square] gap-2 pl-5 marker:text-accent">{children}</ul>;
}

function Code({ children }: { children: React.ReactNode }) {
  return <code className="bg-surface-2 px-1.5 py-0.5 font-mono text-[0.85em] text-text">{children}</code>;
}

const link = "text-text underline decoration-accent underline-offset-4 hover:text-accent-text";

export default function PrivacyPage() {
  return (
    <article className="relative mx-auto max-w-5xl overflow-hidden px-4 pt-14 pb-24 sm:px-6 md:pt-24">
      <div aria-hidden="true" className="rosette pointer-events-none absolute -top-40 -right-56 size-[36rem] md:-right-40" />
      <header className="relative">
        <p className="label">Effective 7 October 2026</p>
        <h1 className="headline mt-6 text-[clamp(3.25rem,13vw,7rem)]">
          Privacy <em>policy</em>.
        </h1>
        <p className="mt-6 max-w-[48ch] text-lg leading-relaxed text-muted md:text-xl">
          For the Snag desktop app and the Snag browser extension for Chrome, Edge and Firefox.
        </p>
      </header>

      {/* The short version, printed on paper. */}
      <div className="paper relative mt-14 mb-16 border border-line-strong p-1.5 md:mt-20">
        <div className="relative border border-line p-6 sm:p-10">
          <div
            aria-hidden="true"
            className="hatch absolute inset-y-0 right-0 w-1/2 [mask-image:linear-gradient(to_left,black,transparent_85%)]"
          />
          <p className="label relative">In short</p>
          <p className="relative mt-4 text-xl leading-snug font-medium tracking-tight md:text-2xl">
            Snag has no account, no analytics, no ads and no server of its own. The extension sends what it reads only to
            the Snag app on the same computer. Nothing is sent to the developer or to any third party, and nothing is sold.
          </p>
        </div>
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
            galleries), and to look for a newer Snag (at start and every few hours; nothing about you is sent, and you
            can switch it off in Settings → General). An update is only installed when you click Update now, and only if
            it matches the checksum published with it;
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
          <ExternalLink href={REPO_URL} className={link}>
            source code
          </ExternalLink>
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
          <ExternalLink href={ISSUES_URL} className={link}>
            github.com/kuldeepsaini23/snag/issues
          </ExternalLink>
          .
        </p>
      </Section>
    </article>
  );
}
