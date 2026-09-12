import { extractPageOutline } from "../../../lib/content";
import type { OnThisPageProps } from "./OnThisPage.types";
import {
  onThisPageItemVariants,
  onThisPageKickerVariants,
  onThisPageLinkVariants,
  onThisPageListVariants,
  onThisPageVariants,
} from "./OnThisPage.variants";

/**
 * In-article list of section headings that link to heading fragment ids.
 *
 * Locks `public-site.chrome:on-page-toc`. Hidden when the body has no section headings.
 *
 * @param props - Teaching markdown body plus native nav attributes
 * @returns Outline nav, or null when there are no section headings
 */
export function OnThisPage({ className, body, ...props }: OnThisPageProps) {
  const items = extractPageOutline(body);
  if (items.length === 0) {
    return null;
  }

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
            <a href={`#${item.id}`} className={onThisPageLinkVariants()}>
              {item.text}
            </a>
          </li>
        ))}
      </ol>
    </nav>
  );
}
