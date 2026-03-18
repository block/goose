import React from "react";
import BrowserOnly from "@docusaurus/BrowserOnly";
import {useThemeConfig} from "@docusaurus/theme-common";
import {useNavbarMobileSidebar} from "@docusaurus/theme-common/internal";
import NavbarItem from "@theme/NavbarItem";
import DocsLanguageDropdown from "@site/src/components/DocsLanguageDropdown";

function useNavbarItems() {
  return useThemeConfig().navbar.items;
}

export default function NavbarMobilePrimaryMenu() {
  const mobileSidebar = useNavbarMobileSidebar();
  const items = useNavbarItems();

  return (
    <ul className="menu__list">
      <BrowserOnly fallback={null}>
        {() => (
          <DocsLanguageDropdown mobile onClick={() => mobileSidebar.toggle()} />
        )}
      </BrowserOnly>
      {items.map((item, i) => (
        <NavbarItem
          mobile
          {...item}
          onClick={() => mobileSidebar.toggle()}
          key={i}
        />
      ))}
    </ul>
  );
}
