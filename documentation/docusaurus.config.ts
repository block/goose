import { themes as prismThemes } from "prism-react-renderer";
import type { Config } from "@docusaurus/types";
import type * as Preset from "@docusaurus/preset-classic";

// This runs in Node.js - Don't use client-side code here (browser APIs, JSX...)

require('dotenv').config();

const inkeepApiKey = process.env.INKEEP_API_KEY;
const inkeepIntegrationId = process.env.INKEEP_INTEGRATION_ID;
const inkeepOrgId = process.env.INKEEP_ORG_ID;

const config: Config = {
  title: "codename goose",
  tagline:
    "goose is your on-machine AI agent, automating engineering tasks seamlessly within your IDE or terminal",
  favicon: "img/favicon.ico",

  // Set the production url of your site here
  url: "https://block.github.io/",
  // Set the /<baseUrl>/ pathname under which your site is served
  // For GitHub pages deployment, it is often '/<projectName>/'
  baseUrl: "/goose/v1/", // This is temporary for development purposes

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
        },
        theme: {
          customCss: "./src/css/custom.css",
        },
      } satisfies Preset.Options,
    ],
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
          to: "/docs/category/getting-started",
          position: "left",
          label: "Docs",
        },
        { to: "/blog", label: "Blog", position: "left" },
        {
          to: "https://block.github.io/goose/v1/extensions/",
          label: "Extensions",
          position: "left",
        },
        {
          href: "https://discord.gg/block-opensource",
          label: "Discord",
          position: "left",
        },
        {
          href: "https://github.com/block/goose",
          label: "GitHub",
          position: "left",
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
              to: "https://block.github.io/goose/v1/extensions/",
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
              href: "https://www.youtube.com/@goose.videos",
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
              href: "https://bsky.app/profile/block-opensource.bsky.social",
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
          primaryBrandColor: "#1E1E1E"
      },
      aiChatSettings: {
          chatSubjectName: "goose",
          botAvatarSrcUrl: "https://storage.googleapis.com/organization-image-assets/block-botAvatarSrcUrl-1737745528096.png",
          botAvatarDarkSrcUrl: "https://storage.googleapis.com/organization-image-assets/block-botAvatarDarkSrcUrl-1737745527450.png",
          getHelpCallToActions: [
              {
                  name: "GitHub",
                  url: "https://github.com/block/goose",
                  icon: {
                      builtIn: "FaGithub"
                  }
              }
          ],
          quickQuestions: [
              "What is Goose?"
          ]
      }
  },
  } satisfies Preset.ThemeConfig,
  
};

export default config;
