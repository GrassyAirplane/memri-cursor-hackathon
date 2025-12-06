/* Eden.so UI Components */

type CardProps = {
  children: React.ReactNode;
  className?: string;
};

export function Card({ children, className = "" }: CardProps) {
  return (
    <div 
      className={`rounded-[var(--radius-md)] border border-[var(--color-border)] bg-[var(--color-card-bg)] ${className}`}
      style={{
        boxShadow: '0 1px 3px rgba(0,0,0,0.1)',
      }}
    >
      {children}
    </div>
  );
}

type ButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "secondary";
  children: React.ReactNode;
};

export function Button({
  children,
  variant = "primary",
  className = "",
  ...props
}: ButtonProps) {
  const base =
    "button-press rounded-[var(--radius-sm)] px-4 py-2 text-sm font-semibold transition-fast disabled:opacity-50 focus:outline-none";
  const styles =
    variant === "primary"
      ? "bg-[var(--color-primary)] text-white hover:bg-[#E6FFFE] hover:text-[var(--color-primary)]"
      : "bg-[var(--color-text)] text-white hover:bg-[var(--color-text-secondary)]";
  return (
    <button className={`${base} ${styles} ${className}`} {...props}>
      {children}
    </button>
  );
}

type BadgeProps = {
  children: React.ReactNode;
  tone?: "neutral" | "success" | "warn";
};

export function Badge({ children, tone = "neutral" }: BadgeProps) {
  const styles: Record<typeof tone, string> = {
    neutral: "bg-[var(--color-bg-elevated)] text-[var(--color-text-secondary)] border border-[var(--color-border)]",
    success: "bg-green-50 text-green-700 border border-green-200",
    warn: "bg-amber-50 text-amber-700 border border-amber-200",
  };
  return (
    <span 
      className={`rounded-[var(--radius-sm)] px-2 py-1 text-[11px] font-medium ${styles[tone]}`}
      style={{ letterSpacing: '0.05em' }}
    >
      {children}
    </span>
  );
}

