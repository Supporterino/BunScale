import { Link } from '@tanstack/react-router';
import { classNames } from './class-names';
import type {
  NavigationArrangement,
  NavigationDestination,
} from './navigation';

/*
 * `Sidebar` — medium and expanded navigation: the same destinations, as a
 * 260 pt column of 44 pt rows beside the content.
 *
 * This is the second half of the app's single adaptive arrangement. The route
 * tree does not fork per form factor (`tanstack-router-conventions`, "Adaptive
 * layout"): one tree renders, and the *arrangement* changes — tab bar below
 * 768 px, sidebar at and above it. The two primitives therefore take the same
 * destinations array, and the screen passes the same array to both.
 *
 * The active row is marked with `aria-current="page"` (from the router's own
 * active state) and with a subtle background plus the accent as the label
 * colour, so the current destination is not signalled by colour alone; the row
 * is 44 pt tall with a visible focus ring, and hovering is an enhancement only
 * — never the way a row is discovered or activated.
 */

export type SidebarProps = {
  destinations: NavigationDestination[];
  /** Accessible name of the landmark. Defaults to `Primary`. */
  label?: string;
  className?: string;
};

export function Sidebar({
  destinations,
  label = 'Primary',
  className,
}: SidebarProps) {
  const arrangement: NavigationArrangement = 'sidebar';

  return (
    <nav
      aria-label={label}
      className={classNames('bs-sidebar', className)}
      data-navigation-arrangement={arrangement}
    >
      <ul className="bs-sidebar__list">
        {destinations.map((destination) => (
          <li className="bs-sidebar__item" key={destination.label}>
            <Link
              activeProps={{ 'aria-current': 'page' }}
              className="bs-sidebar__link"
              to={destination.to}
            >
              <span aria-hidden="true" className="bs-sidebar__icon">
                {destination.icon}
              </span>
              <span className="bs-sidebar__label">{destination.label}</span>
            </Link>
          </li>
        ))}
      </ul>
    </nav>
  );
}
