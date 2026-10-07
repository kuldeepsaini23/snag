import { cn } from "@/lib/utils";

const variants = {
  primary: "bg-accent text-on-accent hover:brightness-110",
  light: "bg-text text-bg hover:bg-white",
  outline: "border border-line-strong text-text hover:border-text hover:bg-white/5",
};

// Square, mono, uppercase: the site's one button shape.
export function ButtonLink({
  href,
  variant = "primary",
  className,
  children,
}: {
  href: string;
  variant?: keyof typeof variants;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <a
      href={href}
      className={cn(
        "inline-flex h-12 items-center justify-center gap-2.5 px-5 font-mono text-xs font-medium tracking-[0.12em] whitespace-nowrap uppercase transition-[background-color,border-color,filter] active:translate-y-px",
        variants[variant],
        className,
      )}
    >
      {children}
    </a>
  );
}
