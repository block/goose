import React from "react";
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
      <DocsLanguageDropdown mobile onClick={() => mobileSidebar.toggle()} />
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
