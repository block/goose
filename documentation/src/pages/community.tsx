import type { ReactNode } from "react";
import React from "react";
import Link from "@docusaurus/Link";
import useDocusaurusContext from "@docusaurus/useDocusaurusContext";
import Layout from "@theme/Layout";
import Heading from "@theme/Heading";

import styles from "./index.module.css";

// Import community data
import communityConfig from "../data/community/config.json";
import april2025Data from "../data/community/april-2025.json";
import may2025Data from "../data/community/may-2025.json";

// Create a data map for easy access
const communityDataMap = {
  "april-2025": april2025Data,
  "may-2025": may2025Data,
};

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
  const [activeMonth, setActiveMonth] = React.useState(communityConfig.defaultMonth);
  
  const currentData = communityDataMap[activeMonth];

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
      
      {/* Month Tabs */}
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-bottom--lg">
            <div style={{ 
              display: 'flex', 
              justifyContent: 'center', 
              gap: '8px', 
              flexWrap: 'wrap',
              marginBottom: '20px'
            }}>
              {communityConfig.availableMonths.map((month) => (
                <button 
                  key={month.id}
                  className={`button ${activeMonth === month.id ? 'button--primary' : 'button--outline button--primary'}`}
                  onClick={() => setActiveMonth(month.id)}
                >
                  {month.display}
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>
      
      {/* Community Stars */}
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-bottom--md">
            <Heading as="h3">⭐ Community Stars</Heading>
            <p style={{ fontSize: '14px', color: 'var(--ifm-color-emphasis-600)' }}>
              Top 5 Contributors from the open source community!
            </p>
          </div>
        </div>
      </div>
      
      <div className="row">
        {currentData.featuredContributors.map((contributor, index) => (
          <div key={index} className={`col col--2 ${index === 0 ? 'col--offset-1' : ''} margin-bottom--lg`}>
            <div className="card">
              <div className="card__header text--center">
                <div className="avatar avatar--vertical">
                  {contributor.handle !== 'TBD' ? (
                    <img
                      className="avatar__photo avatar__photo--lg"
                      src={`https://github.com/${contributor.handle}.png`}
                      alt={contributor.name}
                    />
                  ) : (
                    <div style={{ 
                      width: '64px', 
                      height: '64px', 
                      borderRadius: '50%', 
                      backgroundColor: '#f0f0f0', 
                      display: 'flex', 
                      alignItems: 'center', 
                      justifyContent: 'center',
                      fontSize: '20px',
                      color: '#666'
                    }}>
                      ?
                    </div>
                  )}
                </div>
              </div>
              <div className="card__body text--center">
                <div className="margin-bottom--sm">
                  <strong>
                    {contributor.handle !== 'TBD' ? (
                      <Link href={`https://github.com/${contributor.handle}`}>
                        {contributor.name} (@{contributor.handle})
                      </Link>
                    ) : (
                      `${contributor.name} (@${contributor.handle})`
                    )}
                  </strong>
                </div>
              </div>
            </div>
          </div>
        ))}
      </div>
      
      {/* Team Stars */}
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-bottom--md">
            <Heading as="h3">⭐ Team Stars</Heading>
            <p style={{ fontSize: '14px', color: 'var(--ifm-color-emphasis-600)' }}>
              Top 5 Contributors across Block Open Source teams!
            </p>
          </div>
        </div>
      </div>
      
      <div className="row">
        {currentData.risingStars.map((contributor, index) => (
          <div key={index} className={`col col--2 ${index === 0 ? 'col--offset-1' : ''} margin-bottom--lg`}>
            <div className="card">
              <div className="card__header text--center">
                <div className="avatar avatar--vertical">
                  {contributor.handle !== 'TBD' ? (
                    <img
                      className="avatar__photo avatar__photo--lg"
                      src={`https://github.com/${contributor.handle}.png`}
                      alt={contributor.name}
                    />
                  ) : (
                    <div style={{ 
                      width: '64px', 
                      height: '64px', 
                      borderRadius: '50%', 
                      backgroundColor: '#f0f0f0', 
                      display: 'flex', 
                      alignItems: 'center', 
                      justifyContent: 'center',
                      fontSize: '20px',
                      color: '#666'
                    }}>
                      ?
                    </div>
                  )}
                </div>
              </div>
              <div className="card__body text--center">
                <div className="margin-bottom--sm">
                  <strong>
                    {contributor.handle !== 'TBD' ? (
                      <Link href={`https://github.com/${contributor.handle}`}>
                        {contributor.name} (@{contributor.handle})
                      </Link>
                    ) : (
                      `${contributor.name} (@${contributor.handle})`
                    )}
                  </strong>
                </div>
              </div>
            </div>
          </div>
        ))}
      </div>
      
      {/* Monthly Leaderboard */}
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-bottom--md">
            <Heading as="h3">🏆 Monthly Leaderboard</Heading>
            <p style={{ fontSize: '14px', color: 'var(--ifm-color-emphasis-600)' }}>
              Rankings of all goose contributors getting loose this month!
            </p>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--6 col--offset-3">
          <div className="card" style={{ padding: '20px' }}>
            <div style={{ 
              display: 'flex', 
              flexDirection: 'column',
              gap: '8px',
              fontSize: '14px'
            }}>
              {currentData.leaderboard.map((contributor, index) => {
                const bgColor = contributor.medal === '🥇' ? '#FFD700' : 
                               contributor.medal === '🥈' ? '#C0C0C0' : 
                               contributor.medal === '🥉' ? '#CD7F32' : null;
                
                return (
                  <div 
                    key={index}
                    style={{ 
                      display: 'flex', 
                      alignItems: 'center', 
                      padding: bgColor ? '12px' : '10px', 
                      backgroundColor: bgColor || '#f8f9fa', 
                      borderRadius: bgColor ? '8px' : '6px', 
                      fontWeight: bgColor ? 'bold' : '500',
                      boxShadow: bgColor ? '0 2px 4px rgba(0,0,0,0.1)' : 'none',
                      borderLeft: !bgColor ? '4px solid #e9ecef' : 'none'
                    }}
                  >
                    {contributor.medal && (
                      <span style={{ marginRight: '12px', fontSize: '18px' }}>
                        {contributor.medal}
                      </span>
                    )}
                    <span style={{ 
                      marginRight: '12px', 
                      minWidth: '30px', 
                      fontSize: bgColor ? '16px' : '14px' 
                    }}>
                      {contributor.rank}.
                    </span>
                    {contributor.handle !== 'TBD' ? (
                      <Link 
                        href={`https://github.com/${contributor.handle}`} 
                        style={{ 
                          color: bgColor ? '#000' : 'inherit',
                          fontSize: bgColor ? '16px' : '14px'
                        }}
                      >
                        @{contributor.handle}
                      </Link>
                    ) : (
                      <span style={{ color: '#999', fontStyle: 'italic' }}>
                        @TBD
                      </span>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-top--lg">
            <p style={{ fontStyle: 'italic', color: 'var(--ifm-color-emphasis-600)', fontSize: '14px' }}>
              Thank you to all our amazing contributors! 🙏
            </p>
          </div>
        </div>
      </div>
      
      {/* Want to be featured section */}
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
                  width: '64px', 
                  height: '64px', 
                  borderRadius: '50%', 
                  backgroundColor: '#f0f0f0', 
                  display: 'flex', 
                  alignItems: 'center', 
                  justifyContent: 'center',
                  fontSize: '20px',
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