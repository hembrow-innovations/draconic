import { useLocation } from "@tanstack/react-router";
import { extractPageOutline } from "../../../lib/content";
import type { OnThisPageProps } from "./OnThisPage.types";
import {
  onThisPageItemVariants,
  onThisPageKickerVariants,
  onThisPageLinkVariants,
  onThisPageListVariants,
  onThisPageVariants,
} from "./OnThisPage.variants";

const currentHeading = { "aria-current": "true" as const };

/**
 * In-article list of section headings that link to heading fragment ids.
 *
 * Locks `public-site.chrome:on-page-toc`. Hidden when the body has no section headings.
 * The heading in view is marked; sibling outline links stay unmarked. Side nav keeps page.
 *
 * @param props - Teaching markdown body plus native nav attributes
 * @returns Outline nav, or null when there are no section headings
 */
export function OnThisPage({ className, body, ...props }: OnThisPageProps) {
  const items = extractPageOutline(body);
  const location = useLocation();
  if (items.length === 0) {
    return null;
  }

  const currentId = location.hash.replace(/^#/, "");

  return (
    <nav
      aria-label="On this page"
      className={onThisPageVariants({ className })}
      {...props}
    >
      <p className={onThisPageKickerVariants()}>On this page</p>
      <ol className={onThisPageListVariants()}>
        {items.map((item) => (
          <li
            key={item.id}
            className={onThisPageItemVariants({
              level: item.level === 3 ? "sub" : "section",
            })}
          >
            <a
              href={`#${item.id}`}
              className={onThisPageLinkVariants()}
              {...(item.id === currentId ? currentHeading : {})}
            >
              {item.text}
            </a>
          </li>
        ))}
      </ol>
    </nav>
  );
}
