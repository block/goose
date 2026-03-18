import React, {useEffect, useMemo, useRef, useState} from "react";
import Link from "@docusaurus/Link";
import useDocusaurusContext from "@docusaurus/useDocusaurusContext";
import {useLocation} from "@docusaurus/router";
import DropdownNavbarItem from "@theme/NavbarItem/DropdownNavbarItem";
import clsx from "clsx";
import styles from "./styles.module.css";

type LanguageKey = "en" | "zh-CN";

type LanguageOption = {
  key: LanguageKey;
  label: string;
  menuLabel?: string;
  flag: string;
};

const LANGUAGE_OPTIONS: LanguageOption[] = [
  {key: "en", label: "English", flag: "🇺🇸"},
  {
    key: "zh-CN",
    label: "简体中文",
    menuLabel: "简体中文（预览）",
    flag: "🇨🇳",
  },
];

const DOCS_HOME: Record<LanguageKey, string> = {
  en: "/docs/quickstart",
  "zh-CN": "/zh-CN/docs/quickstart",
};

function normalizePath(path: string): string {
  if (!path) {
    return "/";
  }
  if (path.length > 1 && path.endsWith("/")) {
    return path.slice(0, -1);
  }
  return path;
}

function stripBaseUrl(pathname: string, baseUrl: string): string {
  const normalizedBaseUrl =
    baseUrl.length > 1 && baseUrl.endsWith("/")
      ? baseUrl.slice(0, -1)
      : baseUrl;

  if (pathname === normalizedBaseUrl) {
    return "/";
  }

  if (pathname.startsWith(`${normalizedBaseUrl}/`)) {
    return pathname.slice(normalizedBaseUrl.length);
  }

  return pathname;
}

function getCurrentLanguage(relativePath: string): LanguageKey {
  return relativePath.startsWith("/zh-CN/docs") ? "zh-CN" : "en";
}

function isDocsPath(relativePath: string): boolean {
  return (
    relativePath === "/docs" ||
    relativePath.startsWith("/docs/") ||
    relativePath === "/zh-CN/docs" ||
    relativePath.startsWith("/zh-CN/docs/")
  );
}

function getTargetDocsPath(
  _relativePath: string,
  targetLanguage: LanguageKey,
): string {
  return DOCS_HOME[targetLanguage];
}

export default function DocsLanguageDropdownNavbarItem({
  mobile = false,
  onClick,
}: {
  mobile?: boolean;
  onClick?: () => void;
}) {
  const {siteConfig} = useDocusaurusContext();
  const location = useLocation();
  const dropdownRef = useRef<HTMLDivElement | null>(null);
  const [isOpen, setIsOpen] = useState(false);

  const relativePath = useMemo(
    () => normalizePath(stripBaseUrl(location.pathname, siteConfig.baseUrl)),
    [location.pathname, siteConfig.baseUrl],
  );

  if (!isDocsPath(relativePath)) {
    return null;
  }

  const currentLanguage = getCurrentLanguage(relativePath);

  const items = useMemo(() => {
    return LANGUAGE_OPTIONS.map((option) => {
      const basePath = getTargetDocsPath(relativePath, option.key);
      const to = basePath;
      const isCurrent = option.key === currentLanguage;

      return {
        ...option,
        to,
        isCurrent,
        mobileLabel: `${option.flag} ${option.menuLabel ?? option.label}`,
        desktopLabel: `${option.flag} ${option.menuLabel ?? option.label}${isCurrent ? " ✓" : ""}`,
      };
    });
  }, [currentLanguage, relativePath]);

  useEffect(() => {
    if (!isOpen) {
      return undefined;
    }

    function handleClickOutside(event: MouseEvent | TouchEvent) {
      if (
        dropdownRef.current &&
        event.target instanceof Node &&
        !dropdownRef.current.contains(event.target)
      ) {
        setIsOpen(false);
      }
    }

    function handleEscape(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setIsOpen(false);
      }
    }

    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("touchstart", handleClickOutside);
    document.addEventListener("keydown", handleEscape);

    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("touchstart", handleClickOutside);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [isOpen]);

  if (mobile) {
    return (
      <DropdownNavbarItem
        mobile
        onClick={onClick}
        label={`🌐 ${
          LANGUAGE_OPTIONS.find((option) => option.key === currentLanguage)
            ?.label ?? "Language"
        }`}
        items={items.map((item) => ({
          label: item.mobileLabel,
          to: item.to,
          target: "_self",
          className: item.isCurrent ? "menu__link--active" : "",
        }))}
      />
    );
  }

  const currentOption =
    LANGUAGE_OPTIONS.find((option) => option.key === currentLanguage) ??
    LANGUAGE_OPTIONS[0];

  return (
    <div ref={dropdownRef} className={clsx("navbar__item", styles.wrapper)}>
      <button
        type="button"
        className={styles.trigger}
        aria-haspopup="menu"
        aria-expanded={isOpen}
        aria-label="Switch docs language"
        onClick={() => setIsOpen((open) => !open)}>
        <span className={styles.flag} aria-hidden="true">
          {currentOption.flag}
        </span>
        <span className={styles.label}>{currentOption.label}</span>
        <span
          className={clsx(styles.chevron, {
            [styles.chevronOpen]: isOpen,
          })}
          aria-hidden="true">
          ▾
        </span>
      </button>

      {isOpen ? (
        <ul className={styles.menu} role="menu">
          {items.map((item) => (
            <li key={item.key} className={styles.menuItemWrapper} role="none">
              <Link
                to={item.to}
                className={clsx(styles.menuItem, {
                  [styles.menuItemActive]: item.isCurrent,
                })}
                role="menuitem"
                onClick={() => setIsOpen(false)}>
                <span className={styles.flag} aria-hidden="true">
                  {item.flag}
                </span>
                <span className={styles.label}>{item.label}</span>
                {item.isCurrent ? (
                  <span className={styles.check} aria-hidden="true">
                    ✓
                  </span>
                ) : null}
              </Link>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
