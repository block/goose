import React, { useState } from "react";
import OriginalNavbar from "@theme-original/Navbar";
import { useLocation } from "@docusaurus/router";
import { useColorMode } from "@docusaurus/theme-common";
import useBaseUrl from "@docusaurus/useBaseUrl";
import Link from "@docusaurus/Link";
import FullScreenMenu from "@site/src/components/FullScreenMenu/FullScreenMenu";
import styles from "./styles.module.css";

function isHomepage(pathname: string): boolean {
  return pathname === "/" || pathname === "/goose/" || pathname === "/goose";
}

function HomepageNavbar(): React.JSX.Element {
  const { colorMode } = useColorMode();
  const [menuOpen, setMenuOpen] = useState(false);
  const logoLight = useBaseUrl("/img/logo_light.png");
  const logoDark = useBaseUrl("/img/logo_dark.png");
  const logoSrc = colorMode === "dark" ? logoDark : logoLight;

  return (
    <>
      <nav className={styles.customNavbar}>
        <Link to="/" className={styles.logo}>
          <img src={logoSrc} alt="goose" className={styles.logoImage} />
        </Link>
        <button
          className={styles.menuButton}
          onClick={() => setMenuOpen(true)}
          aria-label="Open menu"
        >
          Menu +
        </button>
      </nav>
      <FullScreenMenu isOpen={menuOpen} onClose={() => setMenuOpen(false)} />
    </>
  );
}

export default function NavbarWrapper(): React.JSX.Element {
  const { pathname } = useLocation();

  if (isHomepage(pathname)) {
    return <HomepageNavbar />;
  }

  return <OriginalNavbar />;
}
