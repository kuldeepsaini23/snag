import { cn } from "@/lib/utils";

// Snag's segment map: one file split into eight parts that download side by side,
// each at its own pace, like the map in the app's details panel.
const parts = [
  { duration: 1.5, delay: 0.2 },
  { duration: 2.1, delay: 0.25 },
  { duration: 1.2, delay: 0.3 },
  { duration: 2.4, delay: 0.25 },
  { duration: 1.8, delay: 0.35 },
  { duration: 1.4, delay: 0.4 },
  { duration: 2.2, delay: 0.3 },
  { duration: 1.7, delay: 0.45 },
];

// A word that downloads: eight vertical slices fill left to right in parallel, then the
// gaps close and it settles into solid type. The real text stays underneath for readers.
export function SegmentedWord({ text, className }: { text: string; className?: string }) {
  return (
    <span className={cn("seg-word", className)}>
      <span className="seg-word__base">{text}</span>
      {parts.map((part, i) => (
        <span
          key={i}
          aria-hidden="true"
          className="seg-word__slice"
          style={
            {
              "--l": `${i * 12.5}%`,
              "--r": `${100 - (i + 1) * 12.5}%`,
              "--d": `${part.duration}s`,
              "--delay": `${part.delay}s`,
            } as React.CSSProperties
          }
        >
          {text}
        </span>
      ))}
      {parts.slice(1).map((_, i) => (
        <span key={i} aria-hidden="true" className="seg-word__gap" style={{ left: `${(i + 1) * 12.5}%` }} />
      ))}
    </span>
  );
}

// The same map on a loop: a file comes down, finishes, and the next one starts.
const loop = [3.2, 4.1, 2.6, 4.6, 3.6, 2.9, 4.3, 3.4];

export function SegmentLoop({ className }: { className?: string }) {
  return (
    <div className={cn("flex gap-1", className)} aria-hidden="true">
      {loop.map((duration, i) => (
        <span key={i} className="h-full flex-1 overflow-hidden rounded-[3px] bg-raised">
          <span
            className="block h-full origin-left animate-fill-loop bg-accent"
            style={{ "--fill-duration": `${duration}s` } as React.CSSProperties}
          />
        </span>
      ))}
    </div>
  );
}
