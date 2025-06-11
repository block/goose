import type { ReactNode } from "react";
import Link from "@docusaurus/Link";
import useDocusaurusContext from "@docusaurus/useDocusaurusContext";
import Layout from "@theme/Layout";
import Heading from "@theme/Heading";

import styles from "./index.module.css";

function CommunityHeader() {
  return (
    <header className={styles.header}>
      <div className={styles.wrapper}>
        <div className={styles.textColumn}>
          <Heading as="h1">Join the Goose Community</Heading>
          <p className={styles.subtitle}>
            Connect with developers, share your experiences, and help shape the future of Goose
          </p>
        </div>
      </div>
    </header>
  );
}

function CommunitySection() {
  return (
    <section className="container margin-vert--lg">
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-bottom--lg">
            <Heading as="h2">Get Involved</Heading>
            <p>There are many ways to connect with the Goose community and contribute to the project.</p>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--6 margin-bottom--lg">
          <div className="card">
            <div className="card__header">
              <Heading as="h3">💬 Discord</Heading>
            </div>
            <div className="card__body">
              <p>
                Join our Discord server to chat with other users, get help, share your projects, 
                and stay up to date with the latest developments.
              </p>
              <Link 
                className="button button--primary" 
                href="https://discord.gg/block-opensource"
              >
                Join Discord
              </Link>
            </div>
          </div>
        </div>
        
        <div className="col col--6 margin-bottom--lg">
          <div className="card">
            <div className="card__header">
              <Heading as="h3">🐙 GitHub</Heading>
            </div>
            <div className="card__body">
              <p>
                Contribute to Goose development, report bugs, request features, 
                and explore the source code on GitHub.
              </p>
              <Link 
                className="button button--primary" 
                href="https://github.com/block/goose"
              >
                View on GitHub
              </Link>
            </div>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--6 margin-bottom--lg">
          <div className="card">
            <div className="card__header">
              <Heading as="h3">📺 YouTube</Heading>
            </div>
            <div className="card__body">
              <p>
                Watch tutorials, demos, and community showcases on our YouTube channel 
                to learn more about Goose capabilities.
              </p>
              <Link 
                className="button button--primary" 
                href="https://www.youtube.com/@blockopensource"
              >
                Watch Videos
              </Link>
            </div>
          </div>
        </div>
        
        <div className="col col--6 margin-bottom--lg">
          <div className="card">
            <div className="card__header">
              <Heading as="h3">📝 Blog</Heading>
            </div>
            <div className="card__body">
              <p>
                Read the latest updates, tutorials, and community stories on our blog 
                to stay informed about Goose developments.
              </p>
              <Link 
                className="button button--primary" 
                to="/blog"
              >
                Read Blog
              </Link>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

function ContributeSection() {
  return (
    <section className="container margin-vert--lg">
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-bottom--lg">
            <Heading as="h2">Ways to Contribute</Heading>
            <p>Help make Goose better for everyone</p>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--4 margin-bottom--lg">
          <div className="text--center">
            <Heading as="h3">🔧 Code Contributions</Heading>
            <p>
              Submit pull requests, fix bugs, add features, or improve documentation. 
              Check out our contributing guidelines to get started.
            </p>
          </div>
        </div>
        
        <div className="col col--4 margin-bottom--lg">
          <div className="text--center">
            <Heading as="h3">🐛 Bug Reports</Heading>
            <p>
              Found a bug? Report it on GitHub with detailed steps to reproduce. 
              Your reports help us improve Goose for everyone.
            </p>
          </div>
        </div>
        
        <div className="col col--4 margin-bottom--lg">
          <div className="text--center">
            <Heading as="h3">💡 Feature Ideas</Heading>
            <p>
              Have an idea for a new feature? Share it on GitHub discussions 
              or Discord to get feedback from the community.
            </p>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--4 margin-bottom--lg">
          <div className="text--center">
            <Heading as="h3">📚 Documentation</Heading>
            <p>
              Help improve our docs by fixing typos, adding examples, 
              or writing new guides based on your experience.
            </p>
          </div>
        </div>
        
        <div className="col col--4 margin-bottom--lg">
          <div className="text--center">
            <Heading as="h3">🤝 Help Others</Heading>
            <p>
              Answer questions on Discord, share your knowledge, 
              and help newcomers get started with Goose.
            </p>
          </div>
        </div>
        
        <div className="col col--4 margin-bottom--lg">
          <div className="text--center">
            <Heading as="h3">📢 Spread the Word</Heading>
            <p>
              Share your Goose projects, write blog posts, or give talks 
              about how Goose has helped you be more productive.
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}

function SocialLinksSection() {
  return (
    <section className="container margin-vert--lg">
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-bottom--lg">
            <Heading as="h2">Follow Us</Heading>
            <p>Stay connected across all platforms</p>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--12">
          <div className="text--center">
            <div className="margin-bottom--md">
              <Link 
                className="button button--outline button--primary margin-horiz--sm" 
                href="https://discord.gg/block-opensource"
              >
                Discord
              </Link>
              <Link 
                className="button button--outline button--primary margin-horiz--sm" 
                href="https://github.com/block/goose"
              >
                GitHub
              </Link>
              <Link 
                className="button button--outline button--primary margin-horiz--sm" 
                href="https://www.youtube.com/@blockopensource"
              >
                YouTube
              </Link>
              <Link 
                className="button button--outline button--primary margin-horiz--sm" 
                href="https://www.linkedin.com/company/block-opensource"
              >
                LinkedIn
              </Link>
            </div>
            <div>
              <Link 
                className="button button--outline button--primary margin-horiz--sm" 
                href="https://x.com/blockopensource"
              >
                Twitter / X
              </Link>
              <Link 
                className="button button--outline button--primary margin-horiz--sm" 
                href="https://bsky.app/profile/opensource.block.xyz"
              >
                BlueSky
              </Link>
              <Link 
                className="button button--outline button--primary margin-horiz--sm" 
                href="https://njump.me/opensource@block.xyz"
              >
                Nostr
              </Link>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

export default function Community(): ReactNode {
  const { siteConfig } = useDocusaurusContext();
  return (
    <Layout 
      title="Community" 
      description="Join the Goose community - connect with developers, contribute to the project, and help shape the future of AI-powered development tools."
    >
      <CommunityHeader />
      <main>
        <CommunitySection />
        <ContributeSection />
        <SocialLinksSection />
      </main>
    </Layout>
  );
}