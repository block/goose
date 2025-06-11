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
            <Heading as="h1">Upcoming Events</Heading>
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
            <Heading as="h1">Community All Stars</Heading>
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
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>
                  <Link href="https://github.com/blackgirlbytes">
                    Rizel Scarlett (@blackgirlbytes)
                  </Link>
                </strong>
                <br />
                <small>Developer Advocate</small>
              </div>
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
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>
                  <Link href="https://github.com/zanesq">
                    Zane Squires (@zanesq)
                  </Link>
                </strong>
                <br />
                <small>Senior Software Engineer</small>
              </div>
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
                  alt="The-Best-Codes"
                />
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>
                  <Link href="https://github.com/The-Best-Codes">
                    The-Best-Codes (@The-Best-Codes)
                  </Link>
                </strong>
                <br />
                <small>Open Source Developer</small>
              </div>
            </div>
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
                  src="https://github.com/michaelneale.png"
                  alt="Michael Neale"
                />
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>
                  <Link href="https://github.com/michaelneale">
                    Michael Neale (@michaelneale)
                  </Link>
                </strong>
                <br />
                <small>Principal Engineer</small>
              </div>
            </div>
          </div>
        </div>
        
        <div className="col col--4 margin-bottom--lg">
          <div className="card">
            <div className="card__header text--center">
              <div className="avatar avatar--vertical">
                <img
                  className="avatar__photo avatar__photo--xl"
                  src="https://github.com/patrickReiis.png"
                  alt="Patrick Reis"
                />
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>
                  <Link href="https://github.com/patrickReiis">
                    Patrick Reis (@patrickReiis)
                  </Link>
                </strong>
                <br />
                <small>Community Contributor</small>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-bottom--lg">
            <Heading as="h2">Want to be featured?</Heading>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--4 col--offset-4 margin-bottom--lg">
          <div className="card">
            <div className="card__header text--center">
              <div className="avatar avatar--vertical">
                <div style={{ 
                  width: '96px', 
                  height: '96px', 
                  borderRadius: '50%', 
                  backgroundColor: '#f0f0f0', 
                  display: 'flex', 
                  alignItems: 'center', 
                  justifyContent: 'center',
                  fontSize: '24px',
                  color: '#666'
                }}>
                  ?
                </div>
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>Your Name Here</strong>
                <br />
                <small>Future Community Star</small>
              </div>
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