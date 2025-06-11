import type { ReactNode } from "react";
import Link from "@docusaurus/Link";
import useDocusaurusContext from "@docusaurus/useDocusaurusContext";
import Layout from "@theme/Layout";
import Heading from "@theme/Heading";

import styles from "./index.module.css";



function UpcomingEventsSection() {
  return (
    <section className="container margin-vert--lg">
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-bottom--lg">
            <Heading as="h2">Upcoming Events</Heading>
            <p>Join us for community events, workshops, and discussions about Goose.</p>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--12 margin-bottom--lg">
          <div className="card">
            <div className="card__header">
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <Heading as="h3">🗓️ No Events Scheduled</Heading>
                <span className="badge badge--secondary">Coming Soon</span>
              </div>
            </div>
            <div className="card__body">
              <p>
                We're currently planning exciting community events! Check back soon for upcoming 
                workshops, AMAs, and community showcases. In the meantime, join our Discord 
                to stay updated and suggest event ideas.
              </p>
              <div style={{ display: 'flex', gap: '10px', flexWrap: 'wrap' }}>
                <Link 
                  className="button button--primary" 
                  href="https://discord.gg/block-opensource"
                >
                  Join Discord for Updates
                </Link>
                <Link 
                  className="button button--outline button--primary" 
                  to="/blog"
                >
                  Read Latest News
                </Link>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--12">
          <div className="text--center">
            <p style={{ fontStyle: 'italic', color: 'var(--ifm-color-emphasis-600)' }}>
              Want to organize a community event or have ideas for workshops? 
              Reach out to us on <Link href="https://discord.gg/block-opensource">Discord</Link> or 
              create a discussion on <Link href="https://github.com/block/goose/discussions">GitHub</Link>.
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}

function CommunityAllStarsSection() {
  return (
    <section className="container margin-vert--lg">
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-bottom--lg">
            <Heading as="h2">Community All Stars</Heading>
            <p>Celebrating our most active contributors and community champions.</p>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--4 margin-bottom--lg">
          <div className="card">
            <div className="card__header text--center">
              <div className="avatar avatar--vertical">
                <img
                  className="avatar__photo avatar__photo--xl"
                  src="https://github.com/blackgirlbytes.png"
                  alt="Rizel Scarlett"
                />
                <div className="avatar__intro">
                  <div className="avatar__name">🌟 Featured Contributors</div>
                  <small className="avatar__subtitle">Community Champions</small>
                </div>
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>Rizel Scarlett (@blackgirlbytes)</strong>
                <br />
                <small>Developer Advocate</small>
              </div>
              <p>
                Leading community engagement and helping developers build amazing projects with Goose, 
                including Google Docs extensions and creative automation solutions.
              </p>
              <Link 
                className="button button--outline button--primary button--sm" 
                href="https://github.com/blackgirlbytes"
              >
                View Profile
              </Link>
            </div>
          </div>
        </div>
        
        <div className="col col--4 margin-bottom--lg">
          <div className="card">
            <div className="card__header text--center">
              <div className="avatar avatar--vertical">
                <img
                  className="avatar__photo avatar__photo--xl"
                  src="https://github.com/zanesq.png"
                  alt="Zane Squires"
                />
                <div className="avatar__intro">
                  <div className="avatar__name">🏆 Top Contributors</div>
                  <small className="avatar__subtitle">This Month</small>
                </div>
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>Top Internal Contributors:</strong>
                <br />
                <small>zanesq • michaelneale • angiejones • Kvadratni</small>
              </div>
              <div className="margin-bottom--sm">
                <strong>Top External Contributors:</strong>
                <br />
                <small>Murf • bwalding • acheong08 • patrickReiis</small>
              </div>
              <p>
                Our most active contributors this month, driving code improvements, 
                bug fixes, and feature development.
              </p>
            </div>
          </div>
        </div>
        
        <div className="col col--4 margin-bottom--lg">
          <div className="card">
            <div className="card__header text--center">
              <div className="avatar avatar--vertical">
                <img
                  className="avatar__photo avatar__photo--xl"
                  src="https://github.com/The-Best-Codes.png"
                  alt="Rising Star"
                />
                <div className="avatar__intro">
                  <div className="avatar__name">⭐ Rising Stars</div>
                  <small className="avatar__subtitle">New Contributors</small>
                </div>
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>Recent Rising Stars:</strong>
                <br />
                <small>The-Best-Codes • svenstaro • faces-of-eth • wesbillman</small>
              </div>
              <p>
                Welcoming new community members who are making their first 
                meaningful contributions to the Goose ecosystem and showing great potential.
              </p>
              <Link 
                className="button button--outline button--primary button--sm" 
                href="https://github.com/block/goose/graphs/contributors"
              >
                View All Contributors
              </Link>
            </div>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--12">
          <div className="text--center">
            <p style={{ fontStyle: 'italic', color: 'var(--ifm-color-emphasis-600)' }}>
              Want to be featured as a Community All Star? Start contributing on{' '}
              <Link href="https://github.com/block/goose">GitHub</Link>, help others on{' '}
              <Link href="https://discord.gg/block-opensource">Discord</Link>, or share your 
              Goose projects with the community!
            </p>
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
      <main>
        <UpcomingEventsSection />
        <CommunityAllStarsSection />
      </main>
    </Layout>
  );
}