import { openUrl } from "@tauri-apps/plugin-opener";
import type { ReactNode, MouseEvent } from "react";

interface ExternalLinkProps {
  href: string;
  children: ReactNode;
  className?: string;
}

/**
 * A link component that opens URLs in the system browser.
 * In Tauri apps, regular <a> tags with target="_blank" don't work as expected.
 * This component uses tauri-plugin-opener to open links externally.
 */
export function ExternalLink({ href, children, className }: ExternalLinkProps) {
  const handleClick = (e: MouseEvent<HTMLAnchorElement>) => {
    e.preventDefault();
    openUrl(href);
  };

  return (
    <a
      href={href}
      onClick={handleClick}
      className={className}
      rel="noopener noreferrer"
    >
      {children}
    </a>
  );
}
