import { cn } from "@/lib/utils";

// Snag's segment map: one file split into parts that download side by side.
// Each part fills at its own pace, like the map in the app's details panel.
const parts = [
  { duration: 1.5, delay: 0.25 },
  { duration: 2.1, delay: 0.3 },
  { duration: 1.2, delay: 0.35 },
  { duration: 2.4, delay: 0.3 },
  { duration: 1.8, delay: 0.4 },
  { duration: 1.4, delay: 0.45 },
  { duration: 2.2, delay: 0.35 },
  { duration: 1.7, delay: 0.5 },
];

export function SegmentBar({ className }: { className?: string }) {
  return (
    <div className={cn("flex gap-1.5", className)} aria-hidden="true">
      {parts.map((part, i) => (
        <span key={i} className="h-full flex-1 overflow-hidden rounded-[3px] bg-raised">
          <span
            className="block h-full origin-left animate-fill bg-accent"
            style={
              {
                "--fill-duration": `${part.duration}s`,
                "--fill-delay": `${part.delay}s`,
              } as React.CSSProperties
            }
          />
        </span>
      ))}
    </div>
  );
}

// A paused moment from the same map: how far each part has got.
export function SegmentMap({ progress, className }: { progress: number[]; className?: string }) {
  return (
    <div className={cn("flex gap-1", className)} aria-hidden="true">
      {progress.map((p, i) => (
        <span key={i} className="h-full flex-1 overflow-hidden rounded-[3px] bg-raised">
          <span className="block h-full bg-accent" style={{ width: `${p * 100}%` }} />
        </span>
      ))}
    </div>
  );
}
