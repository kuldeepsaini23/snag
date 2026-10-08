import { ShieldCheck } from "@phosphor-icons/react/dist/ssr";

import { AccentPicker } from "@/components/accent-picker";
import {
  CatalogueEntry,
  CatalogueLine,
  Plate,
  Section,
  SectionHead,
  Shot,
  type Entry,
} from "@/components/catalogue";
import { SegmentLoop } from "@/components/segments";

const sites = ["YouTube", "Vimeo", "SoundCloud", "Twitch", "Dailymotion", "Bandcamp", "Reddit", "X", "Instagram", "TikTok", "Bilibili", "Mixcloud"];

// ---- Part 2: downloads ----

const speed: Entry[] = [
  {
    n: 1,
    name: "Eight connections per file",
    meta: "Pause · resume · survives a restart or a crash",
    spec: "8 parts",
    description:
      "Big files are split into up to eight parts that download at once. Pause whenever you like; Snag picks up where it stopped, even after a restart or a crash.",
    demo: (
      <Plate fig={2} caption="one file in eight parts, each at its own pace.">
        <div className="p-4 sm:p-5" aria-hidden="true">
          <div className="flex items-baseline justify-between gap-3 font-mono text-xs">
            <span className="truncate">ubuntu-24.04-desktop-amd64.iso</span>
            <span className="shrink-0 text-muted">8.4 MB/s</span>
          </div>
          <SegmentLoop className="mt-4 h-7" />
          <div className="mt-2 grid grid-cols-8 gap-1 text-center font-display text-xs text-faint italic">
            {["i", "ii", "iii", "iv", "v", "vi", "vii", "viii"].map((n) => (
              <span key={n}>{n}</span>
            ))}
          </div>
          <p className="mt-4 border-t border-line pt-3 font-mono text-[11px] text-faint">Segments · 8 parallel connections</p>
        </div>
      </Plate>
    ),
  },
  {
    n: 2,
    name: "About 35 MB of memory",
    meta: "Written in Rust",
    spec: "~35 MB",
    description: "Snag stays light while big downloads run in the background.",
    demo: (
      <div className="flex items-end gap-4 border-y border-line py-6" aria-hidden="true">
        <span className="font-display text-[clamp(5rem,20vw,8.5rem)] leading-[0.8] font-light tracking-tight italic">35</span>
        <span className="pb-1">
          <span className="block font-display text-2xl italic">megabytes</span>
          <span className="label">of memory, in Rust</span>
        </span>
      </div>
    ),
  },
];

export function Features() {
  return (
    <Section id="features" className="mx-auto max-w-6xl px-4 pt-28 sm:px-6 md:pt-40">
      <SectionHead
        id="features"
        title={
          <>
            Everything it <em>downloads</em>.
          </>
        }
        intro="Everything you’d reach for IDM to do, plus music, galleries, torrents and whole web pages. No account, no licence key."
      />
      <div className="mt-14 border-b border-line md:mt-20">
        {speed.map((entry) => (
          <CatalogueEntry key={entry.n} entry={entry} />
        ))}
      </div>
    </Section>
  );
}

// ---- Part 3: video ----

const video: Entry[] = [
  {
    n: 3,
    name: "Video & music · 1,800+ sites",
    meta: "1080p · 720p · MP3 · playlists · subtitles · live",
    spec: "yt-dlp",
    description:
      "Pick 1080p, 720p or MP3, take a whole playlist with its subtitles, or record a live stream as it airs. Works on more than 1,800 sites, YouTube included.",
    demo: (
      <ul
        className="flex flex-wrap items-baseline gap-x-3 gap-y-1 font-display text-[1.65rem] leading-snug italic md:text-3xl"
        aria-label="Some of the supported sites"
      >
        {sites.map((site, i) => (
          <li key={site} className="flex items-baseline gap-3">
            <span className={i % 2 ? "font-normal" : "font-light"}>{site}</span>
            <span aria-hidden="true" className="text-base text-accent-text">
              ✦
            </span>
          </li>
        ))}
        <li className="text-xl text-faint">and many more</li>
      </ul>
    ),
  },
  {
    n: 4,
    name: "Catches what the player is playing",
    meta: "Browser extension · Chrome, Edge, Firefox",
    spec: "IDM-style",
    description:
      "The extension sees the stream a page’s player loads and puts a Download button on it, with every quality it found, even on sites no list knows about.",
    demo: (
      <Plate window fig={3} caption="the Download button (A), on any video page.">
        <Shot
          name="ext-download-menu"
          widths={[720, 1280]}
          height={800}
          sizes="(min-width: 1024px) 560px, 100vw"
          alt="A video page with the extension’s Download button in its bottom-right corner"
        />
        {/* A callout, as engravers letter the part a plate is about. */}
        <span
          aria-hidden="true"
          className="absolute top-[91%] left-[88.4%] h-[7.6%] w-[11%] rounded-full border border-[#efe9dc] shadow-[0_0_0_4px_rgb(11_11_12/0.6)]"
        />
        <span
          aria-hidden="true"
          className="absolute top-[80%] left-[84%] font-display text-base leading-none text-[#efe9dc] italic sm:text-xl"
        >
          A
        </span>
      </Plate>
    ),
  },
];

export function Video() {
  return (
    <Section id="video" className="mx-auto max-w-6xl px-4 pt-28 sm:px-6 md:pt-40">
      <SectionHead
        id="video"
        title={
          <>
            Catches <em>any</em> video.
          </>
        }
        intro="Like IDM, a Download button appears on videos as they play. Pick a quality and Snag takes it from there."
      />
      <div className="mt-14 border-b border-line md:mt-20">
        {video.map((entry) => (
          <CatalogueEntry key={entry.n} entry={entry} />
        ))}
      </div>
    </Section>
  );
}

// ---- Part 4: the rest of the catalogue ----

const lines: Entry[] = [
  {
    n: 5,
    name: "Galleries and albums",
    meta: "Pinterest · Imgur · Reddit · Instagram · X",
    spec: "gallery-dl",
    description: "Paste a gallery, album or post and Snag brings down every picture in it.",
  },
  {
    n: 6,
    name: "Torrents",
    meta: "Magnet links and .torrent files",
    spec: "built in",
    description: "Magnet links and .torrent files download right in Snag, next to everything else. No second app.",
  },
  {
    n: 7,
    name: "Saved web pages",
    meta: "One .html file you can open offline",
    spec: "1 file",
    description: "Save a whole page, with its pictures and styles, as a single file that opens without the internet.",
  },
  {
    n: 8,
    name: "GitHub repo → ZIP",
    meta: "Paste a repo or branch link",
    spec: ".zip",
    description: "Paste a GitHub repository or branch link and Snag downloads the code as a ZIP.",
  },
];

const tools: Entry[] = [
  {
    n: 9,
    name: "A safety check for programs",
    meta: "VirusTotal · your own free key",
    spec: "SHA-256",
    description:
      "Add your own free VirusTotal key and Snag looks up the programs you download. Only the file’s SHA-256 fingerprint is sent, never the file.",
    demo: (
      <div className="plate" aria-hidden="true">
        <div className="plate__frame">
          <div className="flex items-center gap-3 border-b border-line px-4 py-3.5">
            <ShieldCheck weight="fill" className="size-6 text-accent" />
            <span className="text-sm font-semibold">No engines flagged this file</span>
          </div>
          <div className="flex gap-3 px-4 py-3 font-mono text-xs text-faint">
            <span>SHA-256</span>
            <span className="leader" />
            <span>9f2c…e41a</span>
          </div>
        </div>
      </div>
    ),
  },
  {
    n: 10,
    name: "Rules after download",
    meta: "By type, site or category",
    spec: "automatic",
    description: "Tell Snag what to do once a file lands: convert to MP3, shrink videos, unpack archives or move files.",
    demo: (
      <div className="grid font-mono text-sm" aria-hidden="true">
        {[
          ["Music", "Convert to MP3"],
          ["Videos", "Shrink"],
          ["Archives", "Unpack"],
          ["Programs", "Move to a folder"],
        ].map(([when, then]) => (
          <div key={when} className="flex items-baseline gap-3 border-b border-line py-2.5 first:border-t">
            <span className="text-muted">{when}</span>
            <span className="leader" />
            <span>{then}</span>
          </div>
        ))}
      </div>
    ),
  },
  {
    n: 11,
    name: "Stats",
    meta: "By day, by type, by site",
    spec: "7d · 30d · all",
    description: "See what you downloaded by day, by type and by site, over the last week, month or all time.",
    demo: (
      <Plate window fig={4} caption="the stats page, over the last seven days.">
        <Shot
          name="stats"
          widths={[720, 1382]}
          height={864}
          sizes="(min-width: 1024px) 560px, 100vw"
          alt="Snag’s stats page: data per day, a breakdown by type and the top sites"
        />
      </Plate>
    ),
  },
  {
    n: 12,
    name: "Themes and any accent colour",
    meta: "Light · dark · any colour",
    spec: "yours",
    description: "Make Snag yours. Try it here: pick a colour and this page follows along.",
    demo: <AccentPicker />,
  },
];

export function Catalogue() {
  return (
    <Section id="catalogue" className="mx-auto max-w-6xl px-4 pt-28 sm:px-6 md:pt-40">
      <SectionHead
        id="catalogue"
        title={
          <>
            Pictures, torrents, <em>pages</em>.
          </>
        }
        intro="The rest of the catalogue: what else Snag brings down, and what it does once a file lands."
      />
      <div className="mt-14 md:mt-20">
        {lines.map((entry) => (
          <CatalogueLine key={entry.n} entry={entry} />
        ))}
      </div>
      <div className="border-b border-line">
        {tools.map((entry) => (
          <CatalogueEntry key={entry.n} entry={entry} />
        ))}
      </div>
    </Section>
  );
}
