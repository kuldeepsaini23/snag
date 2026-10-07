import {
  ArrowRight,
  ChartBar,
  DownloadSimple,
  FileHtml,
  FileZip,
  FilmStrip,
  Images,
  Magnet,
  MusicNotes,
  Palette,
  Play,
  ShieldCheck,
  Sliders,
  Subtitles,
  Broadcast,
  Queue,
  Cpu,
  Plus,
} from "@phosphor-icons/react/dist/ssr";

import { AccentPicker } from "@/components/accent-picker";
import { SegmentMap } from "@/components/segment-bar";
import { Marquee } from "@/components/ui/marquee";
import { cn } from "@/lib/utils";

const sites = [
  "YouTube",
  "Vimeo",
  "SoundCloud",
  "Twitch",
  "Dailymotion",
  "Bandcamp",
  "Reddit",
  "X",
  "Instagram",
  "TikTok",
  "Bilibili",
  "Mixcloud",
];

function Card({ className, children }: { className?: string; children: React.ReactNode }) {
  return (
    <article className={cn("relative flex flex-col overflow-hidden rounded-[22px] border border-line bg-surface", className)}>
      {children}
    </article>
  );
}

function CardText({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="p-6 md:p-7">
      <h3 className="heading text-xl md:text-[1.4rem]">{title}</h3>
      <div className="mt-2 max-w-[46ch] leading-relaxed text-muted">{children}</div>
    </div>
  );
}

export function Features() {
  return (
    <section id="features" className="mx-auto max-w-6xl px-4 pt-28 sm:px-6 md:pt-36">
      <h2 className="heading max-w-[18ch] text-4xl md:text-6xl">Everything you’d reach for IDM to do</h2>
      <p className="mt-5 max-w-[56ch] text-lg leading-relaxed text-muted">
        Plus the things it never did: music, galleries, torrents and whole web pages, without an account or a licence key.
      </p>

      <div className="mt-12 grid gap-4 md:grid-cols-6">
        {/* Speed */}
        <Card className="md:col-span-4">
          <CardText title="Eight connections per file">
            Big files are split into up to eight parts that download at once. Pause whenever you like; Snag picks up
            where it stopped, even after a restart or a crash.
          </CardText>
          <div className="mt-auto px-6 pb-7 md:px-7">
            <div className="flex items-baseline justify-between text-sm">
              <span className="font-medium">ubuntu-24.04-desktop-amd64.iso</span>
              <span className="text-muted tabular-nums">8.4 MB/s</span>
            </div>
            <SegmentMap progress={[1, 1, 0.82, 1, 0.64, 0.9, 0.47, 0.71]} className="mt-3 h-5" />
            <p className="mt-2 text-sm text-faint">Segments · 8 parallel connections</p>
          </div>
        </Card>

        {/* Memory */}
        <Card className="justify-between md:col-span-2">
          <div className="p-6 md:p-7">
            <Cpu className="size-7 text-accent" aria-hidden />
          </div>
          <div className="p-6 pt-0 md:p-7 md:pt-0">
            <p className="display text-[4.5rem] text-accent tabular-nums">35 MB</p>
            <h3 className="heading mt-3 text-xl">About this much memory</h3>
            <p className="mt-2 leading-relaxed text-muted">Written in Rust, so it stays light while big downloads run.</p>
          </div>
        </Card>

        {/* Video sites */}
        <Card className="md:col-span-3">
          <CardText title="Video and music from 1,800+ sites">
            Pick 1080p, 720p or MP3, take a whole playlist with its subtitles, or record a live stream as it airs.
          </CardText>
          <ul className="flex flex-wrap gap-2 px-6 md:px-7">
            {[
              { icon: FilmStrip, label: "1080p, 720p…" },
              { icon: MusicNotes, label: "MP3" },
              { icon: Queue, label: "Playlists" },
              { icon: Subtitles, label: "Subtitles" },
              { icon: Broadcast, label: "Live" },
            ].map(({ icon: Icon, label }) => (
              <li key={label} className="flex items-center gap-1.5 rounded-full bg-raised px-3 py-1.5 text-sm">
                <Icon className="size-4 text-accent" aria-hidden />
                {label}
              </li>
            ))}
          </ul>
          <div className="mt-auto grid gap-1 pt-6 pb-5 [mask-image:linear-gradient(90deg,transparent,#000_12%,#000_88%,transparent)]">
            {[sites.slice(0, 6), sites.slice(6)].map((row, i) => (
              <Marquee key={i} reverse={i === 1} className="[--duration:40s] [--gap:0.5rem]" repeat={4}>
                {row.map((site) => (
                  <span key={site} className="rounded-lg border border-line px-3.5 py-2 whitespace-nowrap text-muted">
                    {site}
                  </span>
                ))}
              </Marquee>
            ))}
          </div>
        </Card>

        {/* Extension */}
        <Card className="md:col-span-3">
          <CardText title="Catches what the player is playing">
            The browser extension sees the video stream a page loads and puts a Download button on it, so it works even
            on sites no list knows about.
          </CardText>
          <div className="mt-auto px-6 pb-7 md:px-7" aria-hidden="true">
            <div className="relative aspect-[16/8] overflow-hidden rounded-xl bg-[linear-gradient(135deg,#6b4a1c,#2a2420_70%)]">
              <Play weight="fill" className="absolute top-1/2 left-1/2 size-10 -translate-1/2 text-white/85" />
              <div className="absolute right-3 bottom-3 flex items-center gap-1.5 rounded-full bg-accent py-1.5 pr-3.5 pl-2.5 text-sm font-semibold text-on-accent shadow-lg">
                <DownloadSimple weight="bold" className="size-4" />
                Download
              </div>
              <div className="absolute inset-x-0 bottom-0 h-1 bg-white/15">
                <div className="h-full w-[38%] bg-white/70" />
              </div>
            </div>
          </div>
        </Card>

        {/* More than files */}
        <Card className="md:col-span-2">
          <CardText title="More than files">Snag knows what to do with the links that aren’t a single file.</CardText>
          <ul className="mt-auto grid gap-3 px-6 pb-7 text-[0.95rem] md:px-7">
            <li className="flex gap-3">
              <Images className="mt-0.5 size-5 shrink-0 text-accent" aria-hidden />
              Image galleries from Pinterest, Imgur, Reddit, Instagram and X
            </li>
            <li className="flex gap-3">
              <Magnet className="mt-0.5 size-5 shrink-0 text-accent" aria-hidden />
              Torrents, from magnet links or .torrent files
            </li>
            <li className="flex gap-3">
              <FileHtml className="mt-0.5 size-5 shrink-0 text-accent" aria-hidden />
              Web pages, saved as one file you can open offline
            </li>
            <li className="flex gap-3">
              <FileZip className="mt-0.5 size-5 shrink-0 text-accent" aria-hidden />
              <span>
                GitHub repos <ArrowRight className="inline size-3.5 align-[-0.1em]" aria-label="to" /> a ZIP of the
                branch
              </span>
            </li>
          </ul>
        </Card>

        {/* Rules */}
        <Card className="md:col-span-2">
          <CardText title="Rules after download">
            Tell Snag what to do once a file lands, by type, site or category.
          </CardText>
          <div className="mt-auto grid gap-2 px-6 pb-7 text-sm md:px-7">
            {[
              ["Music", "Convert to MP3"],
              ["Videos", "Shrink"],
              ["Archives", "Unpack"],
              ["Programs", "Move to a folder"],
            ].map(([when, then]) => (
              <div key={when} className="flex items-center gap-2 rounded-lg bg-raised px-3 py-2">
                <Sliders className="size-4 shrink-0 text-faint" aria-hidden />
                <span className="truncate text-muted">{when}</span>
                <ArrowRight className="size-3.5 shrink-0 text-faint" aria-label="then" />
                <span className="ml-auto shrink-0 font-medium">{then}</span>
              </div>
            ))}
          </div>
        </Card>

        {/* VirusTotal */}
        <Card className="md:col-span-2">
          <CardText title="A safety check for programs">
            Add your own free VirusTotal key and Snag looks up the programs you download. Only the file’s SHA-256
            fingerprint is sent, never the file.
          </CardText>
          <div className="mt-auto px-6 pb-7 md:px-7">
            <div className="flex items-center gap-3 rounded-xl border border-line bg-raised p-3.5">
              <ShieldCheck weight="fill" className="size-8 shrink-0 text-accent" aria-hidden />
              <div className="min-w-0 text-sm">
                <p className="font-semibold">No engines flagged this file</p>
                <p className="truncate text-faint">sha256 9f2c…e41a</p>
              </div>
            </div>
          </div>
        </Card>

        {/* Stats */}
        <Card className="md:col-span-4">
          <div className="flex items-start gap-3 p-6 pb-0 md:p-7 md:pb-0">
            <ChartBar className="mt-1 size-6 shrink-0 text-accent" aria-hidden />
            <div>
              <h3 className="heading text-xl md:text-[1.4rem]">Stats</h3>
              <p className="mt-2 leading-relaxed text-muted">What you downloaded by day, by type and by site.</p>
            </div>
          </div>
          <div className="mt-6 ml-6 h-[230px] overflow-hidden rounded-tl-xl border-t border-l border-line sm:h-[300px] md:ml-7 md:h-[330px]">
            <picture>
              <source
                type="image/webp"
                srcSet="/images/stats-760.webp 760w, /images/stats-1382.webp 1382w"
                sizes="(min-width: 768px) 720px, 100vw"
              />
              <img
                src="/images/stats-1382.webp"
                alt="Snag's stats page: data per day, a breakdown by type and the top sites"
                width={1382}
                height={864}
                loading="lazy"
                decoding="async"
                className="block h-auto w-[140%] max-w-none md:w-[115%]"
              />
            </picture>
          </div>
        </Card>

        {/* Themes */}
        <Card className="md:col-span-2">
          <div className="flex items-center gap-3 p-6 md:p-7" aria-hidden="true">
            <Palette className="size-7 text-accent" />
            <span className="ml-auto flex items-center gap-1 rounded-full bg-accent px-3 py-1 text-sm font-semibold text-on-accent">
              <Plus weight="bold" className="size-3.5" />
              Add
            </span>
            <span className="relative h-5 w-9 rounded-full bg-accent">
              <span className="absolute top-0.5 right-0.5 size-4 rounded-full bg-on-accent" />
            </span>
          </div>
          <div className="mt-auto p-6 pt-0 md:p-7 md:pt-0">
            <h3 className="heading text-xl md:text-[1.4rem]">Light, dark, and any accent colour</h3>
            <p className="mt-2 mb-5 leading-relaxed text-muted">Make it yours. Try it here: this page follows along.</p>
            <AccentPicker />
          </div>
        </Card>
      </div>
    </section>
  );
}
