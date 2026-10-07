import {
  AppWindow,
  ChartBar,
  CheckCircle,
  Cpu,
  DownloadSimple,
  FileZip,
  FilmStrip,
  Globe,
  Image as ImageIcon,
  Magnet,
  Palette,
  PuzzlePiece,
  ShieldCheck,
  Sliders,
} from "@phosphor-icons/react/dist/ssr";

import { AccentPicker } from "@/components/accent-picker";
import { RowObserver } from "@/components/row-observer";
import { SegmentLoop } from "@/components/segments";
import { cn } from "@/lib/utils";

type Icon = typeof FilmStrip;

type Row = {
  icon: Icon;
  name: string;
  meta: string;
  size: string;
  description: string;
  demo?: React.ReactNode;
  run?: number;
};

const sites = ["YouTube", "Vimeo", "SoundCloud", "Twitch", "Dailymotion", "Bandcamp", "Reddit", "X", "Instagram", "TikTok", "Bilibili", "Mixcloud"];

function Screenshot({ name, alt, crop }: { name: string; alt: string; crop?: boolean }) {
  const [small, large] = name.startsWith("ext-") ? [720, 1280] : [760, 1382];
  return (
    <div
      className={cn(
        "relative overflow-hidden rounded-lg bg-bg shadow-[0_0_0_1px_var(--line-strong)]",
        crop && "aspect-[16/10]",
      )}
    >
      <picture>
        <source
          type="image/webp"
          srcSet={`/images/${name}-${small}.webp ${small}w, /images/${name}-${large}.webp ${large}w`}
          sizes={crop ? "(min-width: 768px) 900px, 170vw" : "(min-width: 768px) 560px, 100vw"}
        />
        <img
          src={`/images/${name}-${large}.webp`}
          alt={alt}
          width={large}
          height={large === 1280 ? 800 : 864}
          loading="lazy"
          decoding="async"
          className={crop ? "absolute right-0 bottom-0 h-auto w-[170%] max-w-none" : "block h-auto w-full"}
        />
      </picture>
    </div>
  );
}

const groups: { name: string; icon: Icon; rows: Row[] }[] = [
  {
    name: "All downloads",
    icon: DownloadSimple,
    rows: [
      {
        icon: DownloadSimple,
        name: "Eight connections per file",
        meta: "Pause · resume · survives a restart or a crash",
        size: "8 parts",
        description:
          "Big files are split into up to eight parts that download at once. Pause whenever you like; Snag picks up where it stopped, even after a restart or a crash.",
        demo: (
          <div className="rounded-lg bg-bg p-4 shadow-[0_0_0_1px_var(--line)]" aria-hidden="true">
            <div className="flex items-baseline justify-between gap-3 font-mono text-xs">
              <span className="truncate">ubuntu-24.04-desktop-amd64.iso</span>
              <span className="shrink-0 text-muted">8.4 MB/s</span>
            </div>
            <SegmentLoop className="mt-3 h-5" />
            <p className="mt-2 text-xs text-faint">Segments · 8 parallel connections</p>
          </div>
        ),
        run: 1.8,
      },
    ],
  },
  {
    name: "Videos & music",
    icon: FilmStrip,
    rows: [
      {
        icon: FilmStrip,
        name: "Video & music · 1,800+ sites",
        meta: "1080p · 720p · MP3 · playlists · subtitles · live",
        size: "yt-dlp",
        description:
          "Pick 1080p, 720p or MP3, take a whole playlist with its subtitles, or record a live stream as it airs. Works on more than 1,800 sites, YouTube included.",
        demo: (
          <ul className="flex flex-wrap gap-1.5" aria-label="Some of the supported sites">
            {sites.map((site) => (
              <li key={site} className="rounded-md bg-raised px-2.5 py-1 text-sm text-muted">
                {site}
              </li>
            ))}
            <li className="rounded-md px-2.5 py-1 text-sm text-faint">and many more</li>
          </ul>
        ),
        run: 1.5,
      },
      {
        icon: PuzzlePiece,
        name: "Catches what the player is playing",
        meta: "Browser extension · Chrome, Edge, Firefox",
        size: "IDM-style",
        description:
          "The extension sees the stream a page's player loads and puts a Download button on it, with every quality it found, even on sites no list knows about.",
        demo: (
          <Screenshot
            crop
            name="ext-download-menu"
            alt="The extension's Download menu on a video page, listing the stream, MP4, 2160p, 1080p, 720p and MP3 with their sizes"
          />
        ),
        run: 1.3,
      },
    ],
  },
  {
    name: "Images, torrents & pages",
    icon: ImageIcon,
    rows: [
      {
        icon: ImageIcon,
        name: "Galleries and albums",
        meta: "Pinterest · Imgur · Reddit · Instagram · X",
        size: "gallery-dl",
        description: "Paste a gallery, album or post and Snag brings down every picture in it.",
        run: 1.1,
      },
      {
        icon: Magnet,
        name: "Torrents",
        meta: "Magnet links and .torrent files",
        size: "built in",
        description: "Magnet links and .torrent files download right in Snag, next to everything else. No second app.",
        run: 1.6,
      },
      {
        icon: Globe,
        name: "Saved web pages",
        meta: "One .html file you can open offline",
        size: "1 file",
        description: "Save a whole page, with its pictures and styles, as a single file that opens without the internet.",
        run: 1.2,
      },
      {
        icon: FileZip,
        name: "GitHub repo → ZIP",
        meta: "Paste a repo or branch link",
        size: ".zip",
        description: "Paste a GitHub repository or branch link and Snag downloads the code as a ZIP.",
        run: 1.0,
      },
    ],
  },
  {
    name: "Programs",
    icon: AppWindow,
    rows: [
      {
        icon: ShieldCheck,
        name: "A safety check for programs",
        meta: "VirusTotal · your own free key",
        size: "SHA-256",
        description:
          "Add your own free VirusTotal key and Snag looks up the programs you download. Only the file's SHA-256 fingerprint is sent, never the file.",
        demo: (
          <div className="rounded-lg bg-bg shadow-[0_0_0_1px_var(--line)]" aria-hidden="true">
            <div className="flex items-center gap-3 border-b border-line px-4 py-3">
              <ShieldCheck weight="fill" className="size-6 text-accent" />
              <span className="text-sm font-semibold">No engines flagged this file</span>
            </div>
            <div className="flex justify-between px-4 py-2.5 font-mono text-xs text-faint">
              <span>SHA-256</span>
              <span>9f2c…e41a</span>
            </div>
          </div>
        ),
        run: 1.4,
      },
      {
        icon: Sliders,
        name: "Rules after download",
        meta: "By type, site or category",
        size: "automatic",
        description: "Tell Snag what to do once a file lands: convert to MP3, shrink videos, unpack archives or move files.",
        demo: (
          <div className="grid gap-1.5 font-mono text-xs" aria-hidden="true">
            {[
              ["Music", "Convert to MP3"],
              ["Videos", "Shrink"],
              ["Archives", "Unpack"],
              ["Programs", "Move to a folder"],
            ].map(([when, then]) => (
              <div key={when} className="flex items-center gap-2 rounded-md bg-raised px-3 py-2">
                <span className="text-muted">{when}</span>
                <span className="ml-auto text-faint">→</span>
                <span>{then}</span>
              </div>
            ))}
          </div>
        ),
        run: 1.2,
      },
    ],
  },
  {
    name: "Stats & settings",
    icon: ChartBar,
    rows: [
      {
        icon: ChartBar,
        name: "Stats",
        meta: "By day, by type, by site",
        size: "7d · 30d · all",
        description: "See what you downloaded by day, by type and by site, over the last week, month or all time.",
        demo: <Screenshot name="stats" alt="Snag's stats page: data per day, a breakdown by type and the top sites" />,
        run: 1.5,
      },
      {
        icon: Palette,
        name: "Themes and any accent colour",
        meta: "Light · dark · any colour",
        size: "yours",
        description: "Make Snag yours. Try it here: pick a colour and this page follows along.",
        demo: <AccentPicker />,
        run: 1.0,
      },
      {
        icon: Cpu,
        name: "About 35 MB of memory",
        meta: "Written in Rust",
        size: "~35 MB",
        description: "Snag stays light while big downloads run in the background.",
        run: 0.9,
      },
    ],
  },
];

const total = groups.reduce((n, g) => n + g.rows.length, 0);

function DownloadRow({ row }: { row: Row }) {
  const Icon = row.icon;
  return (
    <li className="dl-row rounded-xl transition-colors hover:bg-raised/40" style={{ "--run": `${row.run ?? 1.4}s` } as React.CSSProperties}>
      <div className="flex items-center gap-3 p-3 sm:gap-4">
        <div className="grid h-12 w-14 shrink-0 place-items-center rounded-lg bg-[linear-gradient(135deg,#7a5420,#2e2722_75%)] sm:w-20">
          <Icon weight="fill" className="size-5 text-white/90" aria-hidden />
        </div>
        <div className="min-w-0 flex-1">
          <h3 className="truncate font-medium">{row.name}</h3>
          <p className="truncate font-mono text-xs text-faint">{row.meta}</p>
          <div className="mt-2 h-1 overflow-hidden rounded-full bg-raised">
            <div className="dl-fill h-full rounded-full bg-accent" />
          </div>
        </div>
        <div className="shrink-0 text-right font-mono text-xs sm:w-28">
          <p className="hidden truncate text-muted sm:block">{row.size}</p>
          <p className="sm:mt-0.5">
            <span className="dl-status-run text-faint">
              <span className="hidden sm:inline">Getting…</span>
            </span>
            <span className="dl-status-done inline-flex items-center gap-1 text-accent">
              <CheckCircle weight="fill" className="size-4 sm:size-3.5" aria-hidden />
              <span className="sr-only sm:not-sr-only">Done</span>
            </span>
          </p>
        </div>
      </div>
      <div className="dl-body">
        <div>
          <div
            className={cn(
              "grid gap-5 px-3 pb-5 sm:pl-[6.75rem]",
              row.demo && "md:grid-cols-[minmax(0,1fr)_minmax(0,1.15fr)] md:items-start",
            )}
          >
            <p className="max-w-[52ch] leading-relaxed text-muted">{row.description}</p>
            {row.demo}
          </div>
        </div>
      </div>
    </li>
  );
}

export function Features() {
  return (
    <section id="features" className="mx-auto max-w-6xl px-4 pt-24 sm:px-6 md:pt-32">
      <h2 className="text-4xl font-bold tracking-tight md:text-5xl">Everything it downloads</h2>
      <p className="mt-4 max-w-[56ch] text-lg leading-relaxed text-muted">
        Everything you’d reach for IDM to do, plus music, galleries, torrents and whole web pages. No account, no licence
        key.
      </p>

      <div className="mt-10 rounded-2xl bg-panel p-2 shadow-[0_0_0_1px_var(--line)] sm:p-3">
        <div className="flex items-baseline gap-2 px-3 pt-2 pb-1">
          <span className="font-semibold">Downloads</span>
          <span className="text-sm text-faint">{total} items</span>
        </div>
        {groups.map((group) => {
          const GroupIcon = group.icon;
          return (
            <div key={group.name} className="mt-3">
              <p className="flex items-center gap-2 px-3 py-1.5 text-sm text-faint">
                <GroupIcon className="size-4" aria-hidden />
                {group.name}
                <span className="ml-auto font-mono text-xs">{group.rows.length}</span>
              </p>
              <ul>
                {group.rows.map((row) => (
                  <DownloadRow key={row.name} row={row} />
                ))}
              </ul>
            </div>
          );
        })}
      </div>
      <RowObserver />
    </section>
  );
}
