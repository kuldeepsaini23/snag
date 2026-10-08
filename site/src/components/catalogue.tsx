import { pad, partNumber, parts, type PartId } from "@/lib/parts";
import { cn } from "@/lib/utils";

// The pieces every part of the page is set from: the part's heading, the numbered entries, and
// the plates (pictures in a double rule with an italic caption).

export function Section({
  id,
  className,
  children,
}: {
  id: PartId;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <section id={id} data-part={partNumber(id)} className={cn("relative", className)}>
      {children}
    </section>
  );
}

export function SectionHead({
  id,
  title,
  intro,
  className,
}: {
  id: PartId;
  title: React.ReactNode;
  intro?: React.ReactNode;
  className?: string;
}) {
  const n = partNumber(id);
  return (
    <header className={className} data-reveal>
      <p className="label flex items-center gap-4">
        <span>
          Part <span className="text-text">{pad(n)}</span> of {pad(parts.length)}
        </span>
        <span aria-hidden="true" className="h-px flex-1 bg-line" />
        <span>{parts[n - 1].name}</span>
      </p>
      <h2 className="headline mt-8 text-[clamp(2.75rem,10vw,5.75rem)] md:mt-10">{title}</h2>
      {intro && <p className="mt-6 max-w-[54ch] text-lg leading-relaxed text-muted md:text-xl">{intro}</p>}
    </header>
  );
}

// "No. 04", set the way a catalogue numbers its items.
export function No({ n, className }: { n: number; className?: string }) {
  return (
    <span className={cn("flex items-baseline gap-1.5 font-display italic", className)}>
      <span className="text-base text-faint">No.</span>
      <span className="text-[2.75rem] leading-none font-light tracking-tight tabular-nums md:text-[4.25rem]">{pad(n)}</span>
    </span>
  );
}

export type Entry = {
  n: number;
  name: string;
  meta: string;
  spec: string;
  description: string;
  demo?: React.ReactNode;
};

// One item in the catalogue: its number in the margin, then name, a dotted leader and its spec,
// the description, and a plate or a working sample when it has one.
export function CatalogueEntry({ entry }: { entry: Entry }) {
  return (
    <article
      className="grid gap-x-10 gap-y-4 border-t border-line py-10 md:grid-cols-[10rem_minmax(0,1fr)] md:py-14"
      data-reveal
    >
      <No n={entry.n} />
      <div className="min-w-0">
        <div className="flex items-baseline gap-3">
          <h3 className="text-xl font-semibold tracking-tight md:text-2xl">{entry.name}</h3>
          <span aria-hidden="true" className="leader hidden sm:block" />
          <span className="ml-auto hidden shrink-0 font-mono text-sm text-accent-text sm:block">{entry.spec}</span>
        </div>
        <p className="mt-1.5 font-mono text-xs text-faint">
          <span className="text-accent-text sm:hidden">{entry.spec} · </span>
          {entry.meta}
        </p>
        <div
          className={cn(
            "mt-6 grid gap-8",
            entry.demo && "lg:grid-cols-[minmax(0,0.85fr)_minmax(0,1.15fr)] lg:items-start",
          )}
        >
          <p className="max-w-[52ch] text-[1.0625rem] leading-relaxed text-muted">{entry.description}</p>
          {entry.demo}
        </div>
      </div>
    </article>
  );
}

// A short entry: a line of a price list, for the items that need no picture.
export function CatalogueLine({ entry }: { entry: Entry }) {
  return (
    <article
      className="grid grid-cols-[4.25rem_minmax(0,1fr)] items-baseline gap-x-3 border-t border-line py-7 md:grid-cols-[10rem_minmax(0,1fr)] md:gap-x-10"
      data-reveal
    >
      <span className="flex items-baseline gap-1 font-display italic">
        <span className="text-xs text-faint md:text-sm">No.</span>
        <span className="text-2xl leading-none font-light tabular-nums md:text-4xl">{pad(entry.n)}</span>
      </span>
      <div className="min-w-0">
        <div className="flex items-baseline gap-3">
          <h3 className="text-lg font-semibold tracking-tight md:text-xl">{entry.name}</h3>
          <span aria-hidden="true" className="leader" />
          <span className="shrink-0 font-mono text-sm text-accent-text">{entry.spec}</span>
        </div>
        <p className="mt-2 max-w-[60ch] leading-relaxed text-muted">{entry.description}</p>
        <p className="mt-1.5 font-mono text-xs text-faint">{entry.meta}</p>
      </div>
    </article>
  );
}

export function Plate({
  fig,
  caption,
  className,
  frameClassName,
  children,
}: {
  fig: number;
  caption: React.ReactNode;
  className?: string;
  frameClassName?: string;
  children: React.ReactNode;
}) {
  return (
    <figure className={className}>
      <div className="plate">
        <div className={cn("plate__frame", frameClassName)}>{children}</div>
      </div>
      <figcaption className="mt-3 font-display text-[0.95rem] leading-snug text-muted italic">
        <span className="text-text">Fig. {fig}</span> — {caption}
      </figcaption>
    </figure>
  );
}

// A screenshot as WebP at two widths. `crop` sets it larger than its frame, pinned to a corner.
export function Shot({
  name,
  widths,
  height,
  alt,
  sizes,
  className,
  priority,
}: {
  name: string;
  widths: [number, number] | [number];
  height: number;
  alt: string;
  sizes: string;
  className?: string;
  priority?: boolean;
}) {
  const large = widths[widths.length - 1];
  return (
    <picture>
      <source type="image/webp" srcSet={widths.map((w) => `/images/${name}-${w}.webp ${w}w`).join(", ")} sizes={sizes} />
      <img
        src={`/images/${name}-${large}.webp`}
        alt={alt}
        width={large}
        height={height}
        loading={priority ? "eager" : "lazy"}
        fetchPriority={priority ? "high" : undefined}
        decoding="async"
        className={cn("block h-auto w-full", className)}
      />
    </picture>
  );
}

// The banknote rope between parts.
export function Band({ className }: { className?: string }) {
  return <div aria-hidden="true" className={cn("band", className)} />;
}
