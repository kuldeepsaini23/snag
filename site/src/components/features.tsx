import { DownloadSimple, FileHtml, FileZip, Images, Magnet, ShieldCheck } from "@phosphor-icons/react/dist/ssr";

import { AccentPicker } from "@/components/accent-picker";
import { Section, SectionIntro } from "@/components/section";
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

// One file's parts: how long each takes to fill in the looping demo.
const loopParts = [3.2, 4.1, 2.6, 4.6, 3.6, 2.9, 4.3, 3.4];

function Card({
  label,
  index,
  title,
  description,
  wide,
  media,
}: {
  label: string;
  index: number;
  title: string;
  description: string;
  wide?: boolean;
  media: React.ReactNode;
}) {
  return (
    <article
      data-reveal
      style={{ "--reveal-delay": index % 2 === 0 ? "0.08s" : "0s" } as React.CSSProperties}
      className={cn("flex flex-col bg-bg", wide && "md:col-span-2")}
    >
      <div className="flex h-10 items-center justify-between border-b border-line px-5 sm:px-6">
        <p className="label text-muted">{label}</p>
        <p className="label text-faint">{String(index).padStart(2, "0")}</p>
      </div>
      <div
        className={cn(
          "relative flex items-center justify-center overflow-hidden border-b border-line bg-sunk",
          !wide && "aspect-[16/10]",
        )}
      >
        {media}
      </div>
      <div className="px-5 pt-6 pb-8 sm:px-6">
        <h3 className="font-display text-xl leading-tight font-bold [font-stretch:115%] md:text-[1.35rem]">{title}</h3>
        <p className="mt-2 max-w-[52ch] leading-relaxed text-muted">{description}</p>
      </div>
    </article>
  );
}

// A screenshot filling the card's media area; `zoom` crops in on its bottom-right corner.
function Shot({ name, alt, wide, zoom }: { name: string; alt: string; wide?: boolean; zoom?: boolean }) {
  const [small, large] = name.startsWith("ext-") ? [720, 1280] : [760, 1382];
  return (
    <picture className="block w-full">
      <source
        type="image/webp"
        srcSet={`/images/${name}-${small}.webp ${small}w, /images/${name}-${large}.webp ${large}w`}
        sizes={wide ? "(min-width: 1056px) 1056px, 100vw" : zoom ? "(min-width: 768px) 900px, 170vw" : "(min-width: 768px) 528px, 100vw"}
      />
      <img
        src={`/images/${name}-${large}.webp`}
        alt={alt}
        width={large}
        height={large === 1280 ? 800 : 864}
        loading="lazy"
        decoding="async"
        className={zoom ? "absolute right-0 bottom-0 h-auto w-[170%] max-w-none" : "block h-auto w-full"}
      />
    </picture>
  );
}

export function Features() {
  return (
    <Section id="features" label="01 / Features" note="Real screenshots — captured in Snag">
      <SectionIntro
        title={
          <>
            Stop <em>waiting</em> for downloads.
          </>
        }
      >
        Everything you’d reach for IDM to do, plus music, galleries, torrents and whole web pages. No account, no
        licence key.
      </SectionIntro>

      <div className="grid gap-px border-t border-line bg-line md:grid-cols-2">
        <Card
          label="Speed"
          index={1}
          title="Eight connections per file"
          description="Big files download in up to eight parts at once, and resume where they stopped, even after a restart or a crash."
          media={
            <div className="w-full px-8" aria-hidden="true">
              <div className="flex items-baseline justify-between font-mono text-xs">
                <span className="text-text">ubuntu-24.04-desktop-amd64.iso</span>
                <span className="text-muted">8.4 MB/s</span>
              </div>
              <div className="mt-3 flex h-7 gap-1">
                {loopParts.map((duration, i) => (
                  <span key={i} className="h-full flex-1 bg-raised">
                    <span
                      className="block h-full origin-left animate-fill-loop bg-accent"
                      style={{ "--fill-duration": `${duration}s` } as React.CSSProperties}
                    />
                  </span>
                ))}
              </div>
              <p className="label mt-3 text-faint">Segments · 8 parallel connections</p>
            </div>
          }
        />

        <Card
          label="Video & music"
          index={2}
          title="Video and music from 1,800+ sites"
          description="Pick 1080p, 720p or MP3, take whole playlists with their subtitles, or record a live stream as it airs."
          media={
            <div className="w-full" aria-hidden="true">
              <div className="grid gap-2 [mask-image:linear-gradient(90deg,transparent,#000_15%,#000_85%,transparent)]">
                {[sites.slice(0, 6), sites.slice(6)].map((row, i) => (
                  <Marquee key={i} reverse={i === 1} className="p-0 [--duration:36s] [--gap:0.5rem]" repeat={4}>
                    {row.map((site) => (
                      <span
                        key={site}
                        className="border border-line-strong px-4 py-2.5 font-display text-sm font-bold whitespace-nowrap text-muted [font-stretch:115%]"
                      >
                        {site}
                      </span>
                    ))}
                  </Marquee>
                ))}
              </div>
              <p className="label mt-6 text-center text-faint">1080p · 720p · MP3 · Playlists · Subtitles · Live</p>
            </div>
          }
        />

        <Card
          label="Browser extension"
          index={3}
          title="Catches what the player is playing"
          description="The extension sees the stream a page loads and offers every quality it found, even on sites no list knows about."
          media={
            <Shot
              zoom
              name="ext-download-menu"
              alt="The extension's Download menu on a video page, listing the stream, MP4, 2160p, 1080p, 720p and MP3 with their sizes"
            />
          }
        />

        <Card
          label="Galleries & more"
          index={4}
          title="More than single files"
          description="Image galleries, torrents and magnet links, whole web pages as one offline file, and GitHub repos as a ZIP."
          media={
            <div className="grid w-full grid-cols-2 gap-px self-stretch bg-line" aria-hidden="true">
              {[
                { icon: Images, label: "Galleries" },
                { icon: Magnet, label: "Torrents" },
                { icon: FileHtml, label: "Web pages" },
                { icon: FileZip, label: "Repo → ZIP" },
              ].map(({ icon: Icon, label }) => (
                <div key={label} className="flex flex-col items-center justify-center gap-3 bg-sunk">
                  <Icon className="size-8 text-text" weight="light" />
                  <span className="label text-muted">{label}</span>
                </div>
              ))}
            </div>
          }
        />

        <Card
          label="Automation"
          index={5}
          title="Rules after download"
          description="Convert to MP3, shrink videos, unpack archives or move files, by type, site or category."
          media={
            <div className="grid w-full max-w-sm gap-px border border-line bg-line px-0 font-mono text-xs" aria-hidden="true">
              {[
                ["Music", "Convert to MP3"],
                ["Videos", "Shrink"],
                ["Archives", "Unpack"],
                ["Programs", "Move to a folder"],
              ].map(([when, then]) => (
                <div key={when} className="flex items-center gap-3 bg-sunk px-4 py-3">
                  <span className="text-faint">IF</span>
                  <span className="text-muted">{when}</span>
                  <span className="ml-auto text-faint">→</span>
                  <span className="text-text">{then}</span>
                </div>
              ))}
            </div>
          }
        />

        <Card
          label="Safety"
          index={6}
          title="A safety check for programs"
          description="Add your own free VirusTotal key and Snag looks up the programs you download. Only the file’s SHA-256 is sent."
          media={
            <div className="w-full max-w-sm border border-line-strong bg-bg" aria-hidden="true">
              <div className="flex items-center gap-3 border-b border-line px-4 py-3.5">
                <ShieldCheck weight="fill" className="size-6 text-accent" />
                <span className="text-sm font-semibold">No engines flagged this file</span>
              </div>
              <div className="flex justify-between px-4 py-3 font-mono text-xs text-faint">
                <span>SHA-256</span>
                <span>9f2c…e41a</span>
              </div>
            </div>
          }
        />

        <Card
          label="Stats"
          index={7}
          wide
          title="See where the gigabytes went"
          description="What you downloaded by day, by type and by site, over the last week, month or all time."
          media={
            <Shot
              name="stats"
              wide
              alt="Snag's stats page: data per day, a breakdown by type and the top sites"
            />
          }
        />

        <Card
          label="Footprint"
          index={8}
          title="About 35 MB of memory"
          description="Written in Rust, so it stays light while big downloads run in the background."
          media={
            <p className="font-display text-[clamp(3.5rem,10vw,5.5rem)] leading-none font-bold [font-stretch:125%]" aria-hidden="true">
              35<span className="ml-2 text-[0.45em] text-faint">MB</span>
            </p>
          }
        />

        <Card
          label="Themes"
          index={9}
          title="Light, dark and any accent colour"
          description="Make Snag yours. Try it here: pick a colour and this page follows along."
          media={
            <div className="flex flex-col items-center gap-6 px-6">
              <div className="flex items-center gap-3" aria-hidden="true">
                <span className="flex items-center gap-1.5 bg-accent px-3 py-2 font-mono text-xs text-on-accent uppercase">
                  <DownloadSimple weight="bold" className="size-3.5" />
                  Add
                </span>
                <span className="relative h-5 w-9 bg-accent">
                  <span className="absolute top-0.5 right-0.5 size-4 bg-on-accent" />
                </span>
                <span className="h-1.5 w-24 bg-raised">
                  <span className="block h-full w-2/3 bg-accent" />
                </span>
              </div>
              <AccentPicker />
            </div>
          }
        />
      </div>
    </Section>
  );
}
