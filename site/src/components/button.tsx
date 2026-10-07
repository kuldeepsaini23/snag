import { cn } from "@/lib/utils";

const variants = {
  primary: "bg-accent text-on-accent hover:brightness-110",
  secondary: "bg-raised text-text hover:bg-selected",
};

const sizes = {
  sm: "h-9 px-4 text-sm",
  lg: "h-12 px-6 text-[0.95rem]",
};

// The app's "+ Add" button: a rounded pill, orange for the main action, warm grey otherwise.
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
        "inline-flex items-center justify-center gap-2 rounded-full font-semibold whitespace-nowrap transition-[background-color,filter,transform] active:scale-[0.97]",
        variants[variant],
        sizes[size],
        className,
      )}
    >
      {children}
    </a>
  );
}
