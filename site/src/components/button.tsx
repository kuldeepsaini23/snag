import { cn } from "@/lib/utils";

const variants = {
  primary: "bg-accent text-on-accent hover:bg-accent-hover active:bg-accent-press",
  secondary: "text-text shadow-[inset_0_0_0_1px_var(--line-strong)] hover:bg-surface-2",
};

const sizes = {
  sm: "h-9 px-4 text-sm",
  lg: "h-12 px-6 text-[0.95rem]",
};

// Square-shouldered buttons, like the type blocks of a catalogue: orange for the main action,
// a ruled outline otherwise.
export function ButtonLink({
  href,
  variant = "primary",
  size = "lg",
  className,
  children,
}: {
  href: string;
  variant?: keyof typeof variants;
  size?: keyof typeof sizes;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <a
      href={href}
      className={cn(
        "inline-flex items-center justify-center gap-2.5 rounded-[3px] font-semibold whitespace-nowrap transition-[background-color,filter,transform] active:scale-[0.97]",
        variants[variant],
        sizes[size],
        className,
      )}
    >
      {children}
    </a>
  );
}
