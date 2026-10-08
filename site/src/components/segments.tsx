import { cn } from "@/lib/utils";

// Snag's segment map on a loop: one file in eight parts that download side by side, each at its
// own pace; a file comes down, finishes, and the next one starts. The parts still to come are
// hatched, like the shading of an engraving.
const loop = [3.2, 4.1, 2.6, 4.6, 3.6, 2.9, 4.3, 3.4];

export function SegmentLoop({ className }: { className?: string }) {
  return (
    <div className={cn("flex gap-1", className)} aria-hidden="true">
      {loop.map((duration, i) => (
        <span key={i} className="hatch-1 h-full flex-1 overflow-hidden shadow-[inset_0_0_0_1px_var(--line-strong)]">
          <span
            className="block h-full origin-left animate-fill-loop bg-accent"
            style={{ "--fill-duration": `${duration}s` } as React.CSSProperties}
          />
        </span>
      ))}
    </div>
  );
}
