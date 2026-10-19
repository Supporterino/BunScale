import { Link } from '@tanstack/react-router';
import { classNames } from './class-names';
import type {
  NavigationArrangement,
  NavigationDestination,
} from './navigation';

/*
 * `TabBar` — compact navigation: three to five destinations along the bottom
 * edge, each an icon over a visible label.
 *
 * Contract (design-system inventory):
 *   - `nav` + links, `aria-current="page"` on the active destination;
 *   - icon AND visible label, so the item is never icon-only;
 *   - at least 44 x 44 per item (the bar's own 49 pt content height plus the
 *     bottom safe area, so the row clears the home indicator);
 *   - no more than five destinations — a sixth would make the labels truncate
 *     into ambiguity on a 390 pt-wide screen, and iOS treats five as the limit
 *     for the same reason. This is a design limit, not a runtime error: the
 *     primitive renders what it is given.
 *
 * The links come from the router (`Link`, never an `<a href>`), so navigation
 * is a client-side route change and the active state is the router's own view of
 * the location. `activeProps` is where `aria-current` is set: the state is
 * exposed, not painted, and it is absent rather than `false` on the destinations
 * that are not current.
 *
 * There is no hover-only affordance: every destination is a 44 pt target and the
 * pressed state (`--tab-bar-item-pressed-background`) exists on touch, with a
 * fine-pointer hover as a progressive enhancement in `src/styles/ui.css`.
 */

export type TabBarProps = {
  destinations: NavigationDestination[];
  /** Accessible name of the landmark. Defaults to `Primary`. */
  label?: string;
  className?: string;
};

export function TabBar({
  destinations,
  label = 'Primary',
  className,
}: TabBarProps) {
  const arrangement: NavigationArrangement = 'tabs';

  return (
    <nav
      aria-label={label}
      className={classNames('bs-tab-bar', className)}
      data-navigation-arrangement={arrangement}
    >
      <ul className="bs-tab-bar__list">
        {destinations.map((destination) => (
          <li className="bs-tab-bar__item" key={destination.label}>
            <Link
              activeProps={{ 'aria-current': 'page' }}
              className="bs-tab-bar__link"
              to={destination.to}
            >
              <span aria-hidden="true" className="bs-tab-bar__icon">
                {destination.icon}
              </span>
              <span className="bs-tab-bar__label">{destination.label}</span>
            </Link>
          </li>
        ))}
      </ul>
    </nav>
  );
}
