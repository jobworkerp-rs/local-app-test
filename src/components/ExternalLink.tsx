import { openUrl } from "@tauri-apps/plugin-opener";
import type { ReactNode, MouseEvent } from "react";

interface ExternalLinkProps {
  href: string;
  children: ReactNode;
  className?: string;
}

const ALLOWED_SCHEMES = ["http:", "https:"];

function isAllowedUrl(href: string): boolean {
  try {
    const url = new URL(href);
    return ALLOWED_SCHEMES.includes(url.protocol);
  } catch {
    return false;
  }
}

/**
 * A link component that opens URLs in the system browser.
 * In Tauri apps, regular <a> tags with target="_blank" don't work as expected.
 * This component uses tauri-plugin-opener to open links externally.
 *
 * Security: Only http: and https: URLs are allowed. Other schemes (javascript:,
 * file:, data:, etc.) are blocked and rendered as non-clickable text.
 */
export function ExternalLink({ href, children, className }: ExternalLinkProps) {
  const isAllowed = isAllowedUrl(href);

  const handleClick = (e: MouseEvent<HTMLAnchorElement>) => {
    e.preventDefault();
    if (isAllowed) {
      openUrl(href);
    }
  };

  if (!isAllowed) {
    return <span className={className}>{children}</span>;
  }

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
