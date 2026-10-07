"use client";

import { useRef } from "react";
import { Browser, Globe, HardDrives } from "@phosphor-icons/react";

import { AnimatedBeam } from "@/components/ui/animated-beam";
import { cn } from "@/lib/utils";

function Node({
  ref,
  label,
  detail,
  className,
  children,
}: {
  ref: React.Ref<HTMLDivElement>;
  label: string;
  detail: string;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="relative z-10 flex flex-col items-center gap-2 text-center">
      <div
        ref={ref}
        className={cn("grid size-14 place-items-center rounded-2xl border border-line-strong bg-raised md:size-16", className)}
      >
        {children}
      </div>
      <p className="text-sm font-semibold">{label}</p>
      <p className="-mt-1.5 max-w-[14ch] text-xs text-faint">{detail}</p>
    </div>
  );
}

// Every line that leaves Snag: the extension and the sites you download from come in,
// files go to your own disk. There is no line to a Snag server because there isn't one.
export function PrivacyDiagram() {
  const container = useRef<HTMLDivElement>(null);
  const browser = useRef<HTMLDivElement>(null);
  const sites = useRef<HTMLDivElement>(null);
  const snag = useRef<HTMLDivElement>(null);
  const disk = useRef<HTMLDivElement>(null);

  return (
    <div
      ref={container}
      role="img"
      aria-label="The browser extension and the sites you download from connect only to Snag on your PC, which saves files to your own disk."
      className="relative grid grid-cols-3 items-center gap-4 rounded-[22px] border border-line bg-surface px-4 py-8 md:px-8 md:py-10"
    >
      <div className="flex flex-col gap-10">
        <Node ref={browser} label="Extension" detail="in your browser">
          <Browser className="size-7 text-text" aria-hidden />
        </Node>
        <Node ref={sites} label="Sites" detail="you download from">
          <Globe className="size-7 text-text" aria-hidden />
        </Node>
      </div>
      <Node ref={snag} label="Snag" detail="127.0.0.1, this PC" className="size-18 border-accent/60 md:size-20">
        {/* eslint-disable-next-line @next/next/no-img-element -- static export, already a 96px PNG */}
        <img src="/images/logo-96.png" alt="" width={48} height={48} className="size-11 rounded-xl md:size-12" />
      </Node>
      <Node ref={disk} label="Your disk" detail="files and settings">
        <HardDrives className="size-7 text-text" aria-hidden />
      </Node>

      <AnimatedBeam containerRef={container} fromRef={browser} toRef={snag} curvature={-30} pathColor="var(--line-strong)" gradientStartColor="var(--accent)" gradientStopColor="#ffd28a" duration={4} />
      <AnimatedBeam containerRef={container} fromRef={sites} toRef={snag} curvature={30} pathColor="var(--line-strong)" gradientStartColor="var(--accent)" gradientStopColor="#ffd28a" duration={4} delay={1.2} />
      <AnimatedBeam containerRef={container} fromRef={snag} toRef={disk} pathColor="var(--line-strong)" gradientStartColor="var(--accent)" gradientStopColor="#ffd28a" duration={4} delay={2.2} />
    </div>
  );
}
