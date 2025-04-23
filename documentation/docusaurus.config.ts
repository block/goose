import { themes as prismThemes } from "prism-react-renderer";
import type { Config } from "@docusaurus/types";
import type * as Preset from "@docusaurus/preset-classic";
import tailwindPlugin from "./plugins/tailwind-config.cjs";

// This runs in Node.js - Don't use client-side code here (browser APIs, JSX...)

require("dotenv").config();

const inkeepApiKey = process.env.INKEEP_API_KEY;
const inkeepIntegrationId = process.env.INKEEP_INTEGRATION_ID;
const inkeepOrgId = process.env.INKEEP_ORG_ID;

const config: Config = {
  title: "codename goose",
  tagline:
    "Your on-machine AI agent, automating engineering tasks seamlessly.",
  favicon: "img/favicon.ico",

  // Set the production url of your site here
  url: "https://block.github.io/",
  // Set the /<baseUrl>/ pathname under which your site is served
  // For GitHub pages deployment, it is often '/<projectName>/'
  baseUrl: process.env.TARGET_PATH || "/goose/",

  // GitHub pages deployment config.
  // If you aren't using GitHub pages, you don't need these.
  organizationName: "block", // Usually your GitHub org/user name.
  projectName: "goose", // Usually your repo name.

  onBrokenLinks: "throw",
  onBrokenMarkdownLinks: "warn",

  // Even if you don't use internationalization, you can use this field to set
  // useful metadata like html lang. For example, if your site is Chinese, you
  // may want to replace "en" with "zh-Hans".
  i18n: {
    defaultLocale: "en",
    locales: ["en"],
  },

  presets: [
    [
      "classic",
      {
        docs: {
          sidebarPath: "./sidebars.ts",
        },
        blog: {
          showReadingTime: true,
          feedOptions: {
            type: ["rss", "atom"],
            xslt: true,
          },
          // Useful options to enforce blogging best practices
          onInlineTags: "warn",
          onInlineAuthors: "warn",
          onUntruncatedBlogPosts: "warn",
          blogSidebarCount: 'ALL'
        },
        theme: {
          customCss: [
            "./src/css/custom.css",
            "./src/css/extensions.css",
            "./src/css/tailwind.css",
          ],
        },
      } satisfies Preset.Options,
    ],
  ],
  plugins: [
    [
      "@docusaurus/plugin-client-redirects",
      {
        redirects: [
          {
            from: '/docs/getting-started/using-goose-free',
            to: '/docs/getting-started/providers#using-goose-for-free'
          },
          {
            from: '/v1/docs/getting-started/providers',
            to: '/docs/getting-started/providers'
          },
          {
            from: '/v1/docs/getting-started/installation',
            to: '/docs/getting-started/installation'
          },
          {
            from: '/v1/docs/quickstart',
            to: '/docs/quickstart'
          },
          {
            from: '/v1/',
            to: '/'
          },
          {
            from: '/docs/guides/custom-extensions',
            to: '/docs/tutorials/custom-extensions'
          },
          {
            from: '/docs',
            to: '/docs/category/getting-started'
          },
          {
            from: '/v1/extensions',
            to: '/extensions'
          },
          {
            from: '/docs/guides/share-goose-sessions',
            to: '/docs/guides/session-recipes'
          }
        ],
      },
    ],
    tailwindPlugin,
  ],
  themes: ["@inkeep/docusaurus/chatButton", "@inkeep/docusaurus/searchBar"],
  themeConfig: {
    // Replace with your project's social card
    image: "img/home-banner.png",
    navbar: {
      title: "",
      logo: {
        alt: "Block Logo",
        src: "img/logo_light.png",
        srcDark: "img/logo_dark.png",
      },
      items: [
        {
          to: "/docs/quickstart",
          label: "Quickstart",
          position: "left",
        },
        {
          to: "/extensions",
          label: "Extensions",
          position: "left",
        },
        {
          to: "/docs/category/getting-started",
          position: "left",
          label: "Docs",
        },
        
        {
          to: "/docs/category/tutorials",
          position: "left",
          label: "Tutorials",
        },
        {
          to: "/prompt-library",
          position: "left",
          label: "Prompt Library",
        },
        { to: "/blog", label: "Blog", position: "left" },

        {
          href: "https://discord.gg/block-opensource",
          label: "Discord",
          position: "right",
        },
        {
          href: "https://github.com/block/goose",
          label: "GitHub",
          position: "right",
        },
      ],
    },
    footer: {
      links: [
        {
          title: "Quick Links",
          items: [
            {
              label: "Install Goose",
              to: "docs/getting-started/installation",
            },
            {
              label: "Extensions",
              to: "/extensions",
            },
          ],
        },
        {
          title: "Community",
          items: [
            {
              label: "Discord",
              href: "https://discord.gg/block-opensource",
            },
            {
              label: "YouTube",
              href: "https://www.youtube.com/@blockopensource",
            },
            {
              label: "LinkedIn",
              href: "https://www.linkedin.com/company/block-opensource",
            },
            {
              label: "Twitter / X",
              href: "https://x.com/blockopensource",
            },
            {
              label: "BlueSky",
              href: "https://bsky.app/profile/opensource.block.xyz",
            },
            {
              label: "Nostr",
              href: "https://njump.me/opensource@block.xyz",
            },
          ],
        },
        {
          title: "More",
          items: [
            {
              label: "Blog",
              to: "/blog",
            },
            {
              label: "GitHub",
              href: "https://github.com/block/goose",
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Block, Inc.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.nightOwl,
    },
    inkeepConfig: {
      baseSettings: {
        apiKey: inkeepApiKey,
        integrationId: inkeepIntegrationId,
        organizationId: inkeepOrgId,
        primaryBrandColor: "#1E1E1E",
      },
      aiChatSettings: {
        chatSubjectName: "goose",
        botAvatarSrcUrl:
          "",
        getHelpCallToActions: [
          {
            name: "GitHub",
            url: "https://github.com/block/goose",
            icon: {
              builtIn: "FaGithub",
            },
          },
        ],
        quickQuestions: ["What is Goose?"],
      },
    },
  } satisfies Preset.ThemeConfig,
};

export default config;