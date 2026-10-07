import { cn } from "@/lib/utils";

// The app's "Speed · last minute" graph, stretched across the page. One period of the
// curve is a sum of sines that starts and ends at the same height, drawn twice side by
// side so sliding it left by half loops without a seam.
const W = 1200;
const H = 100;

function curve() {
  const points: string[] = [];
  for (let x = 0; x <= W * 2; x += 12) {
    const t = (x / W) * Math.PI * 2;
    const y = 0.5 + 0.18 * Math.sin(t * 3) + 0.1 * Math.sin(t * 7 + 1) + 0.06 * Math.sin(t * 13 + 2) - 0.12 * Math.sin(t * 2 + 0.5) ** 8;
    points.push(`${x},${(y * H * 0.8 + H * 0.1).toFixed(1)}`);
  }
  return points;
}

const line = curve();
const stroke = `M${line.join(" L")}`;
const area = `${stroke} L${W * 2},${H} L0,${H} Z`;

export function SpeedLine({ id, className, duration = 18 }: { id: string; className?: string; duration?: number }) {
  return (
    <div className={cn("pointer-events-none overflow-hidden", className)} aria-hidden="true">
      <svg
        viewBox={`0 0 ${W * 2} ${H}`}
        preserveAspectRatio="none"
        className="block h-full w-[200%] animate-speed"
        style={{ "--speed-duration": `${duration}s` } as React.CSSProperties}
      >
        <defs>
          <linearGradient id={`${id}-fill`} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" style={{ stopColor: "var(--accent)", stopOpacity: 0.22 }} />
            <stop offset="1" style={{ stopColor: "var(--accent)", stopOpacity: 0 }} />
          </linearGradient>
        </defs>
        <path d={area} fill={`url(#${id}-fill)`} />
        <path d={stroke} fill="none" style={{ stroke: "var(--accent)" }} strokeWidth="2" vectorEffect="non-scaling-stroke" />
      </svg>
    </div>
  );
}
