import { Sun, Moon } from "lucide-react";
import { useTheme } from "../hooks/use-theme";

type Props = {
  className?: string;
  /** Fixed floating position in the top-right of the viewport. */
  floating?: boolean;
};

/**
 * Shared theme toggle. Applies `.dark` on <html> via `useTheme`, persists
 * choice in localStorage, syncs across tabs, and defaults from
 * `prefers-color-scheme: dark`.
 */
export function ThemeToggle({ className = "", floating = false }: Props) {
  const { dark, toggle } = useTheme();
  return (
    <button
      type="button"
      onClick={toggle}
      aria-label={dark ? "Switch to light mode" : "Switch to dark mode"}
      aria-pressed={dark}
      className={`${floating ? "fixed top-4 right-4 z-50 " : ""}surface-inset h-9 w-9 rounded-full grid place-items-center hover:brightness-110 theme-transition ${className}`}
    >
      {dark ? (
        <Sun size={14} className="text-[color:var(--text-body)]" />
      ) : (
        <Moon size={14} className="text-[color:var(--text-body)]" />
      )}
    </button>
  );
}