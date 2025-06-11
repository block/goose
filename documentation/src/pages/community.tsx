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
            <p>Join us for livestreams, workshops, and discussions about goose and open source projects.</p>
          </div>
        </div>
      </div>
      
      {/* Embedded Calendar */}
      <div className="row">
        <div className="col col--12 margin-bottom--lg">
          <iframe
            src="https://calget.com/c/t7jszrie"
            style={{
              width: '100%',
              height: '600px',
              border: 'none',
              borderRadius: '8px'
            }}
            title="Goose Community Calendar"
          />
        </div>
      </div>
      
      {/* Call to Action */}
      <div className="row">
        <div className="col col--12">
          <div className="text--center">
            <p style={{ fontStyle: 'italic', color: 'var(--ifm-color-emphasis-600)' }}>
              Want to join us on a livestream or have ideas for future events? 
              Reach out to us on <Link href="https://discord.gg/block-opensource">Discord</Link>.
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}

function CommunityAllStarsSection() {
  const [activeMonth, setActiveMonth] = React.useState(communityConfig.defaultMonth);
  const [showScrollIndicator, setShowScrollIndicator] = React.useState(true);
  
  const currentData = communityDataMap[activeMonth];

  // Handle scroll to show/hide indicator
  const handleScroll = (e) => {
    const { scrollTop, scrollHeight, clientHeight } = e.target;
    const isAtBottom = scrollTop + clientHeight >= scrollHeight - 10; // 10px threshold
    setShowScrollIndicator(!isAtBottom);
  };

  return (
    <section className="container margin-vert--lg">
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-bottom--lg">
            <Heading as="h1">Community All Stars</Heading>
            <p>Every month we take a moment and celebrate all contributions from the open source community. Here are our top contributors and community champions!</p>
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
                  style={activeMonth === month.id ? {
                    border: '3px solid var(--ifm-color-primary-dark)',
                    boxShadow: '0 2px 8px rgba(0,0,0,0.15)'
                  } : {}}
                >
                  {activeMonth === month.id ? '📅 ' : ''}{month.display}
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
        {currentData.communityStars.map((contributor, index) => (
          <div key={index} className={`col col--2 ${index === 0 ? 'col--offset-1' : ''} margin-bottom--lg`}>
            <div 
              className="card"
              style={{
                transition: 'all 0.3s ease',
                cursor: 'pointer'
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.transform = 'translateY(-8px) scale(1.02)';
                e.currentTarget.style.boxShadow = '0 8px 25px rgba(0,0,0,0.15)';
                e.currentTarget.style.borderColor = 'var(--ifm-color-primary)';
                // Add a little sparkle to the avatar
                const avatar = e.currentTarget.querySelector('.avatar__photo');
                if (avatar) {
                  avatar.style.transform = 'rotate(5deg)';
                }
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.transform = 'translateY(0) scale(1)';
                e.currentTarget.style.boxShadow = '';
                e.currentTarget.style.borderColor = '';
                // Reset avatar rotation
                const avatar = e.currentTarget.querySelector('.avatar__photo');
                if (avatar) {
                  avatar.style.transform = 'rotate(0deg)';
                }
              }}
            >
              <div className="card__header text--center">
                <div className="avatar avatar--vertical">
                  {contributor.avatarUrl ? (
                    <img
                      className="avatar__photo avatar__photo--lg"
                      src={contributor.avatarUrl}
                      alt={contributor.name}
                      style={{ transition: 'transform 0.3s ease' }}
                    />
                  ) : contributor.handle !== 'TBD' ? (
                    <img
                      className="avatar__photo avatar__photo--lg"
                      src={`https://github.com/${contributor.handle}.png`}
                      alt={contributor.name}
                      style={{ transition: 'transform 0.3s ease' }}
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
                      `${contributor.name}`
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
        {currentData.teamStars.map((contributor, index) => (
          <div key={index} className={`col col--2 ${index === 0 ? 'col--offset-1' : ''} margin-bottom--lg`}>
            <div 
              className="card"
              style={{
                transition: 'all 0.3s ease',
                cursor: 'pointer'
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.transform = 'translateY(-8px) scale(1.02)';
                e.currentTarget.style.boxShadow = '0 8px 25px rgba(0,0,0,0.15)';
                e.currentTarget.style.borderColor = 'var(--ifm-color-primary)';
                // Add a little sparkle to the avatar
                const avatar = e.currentTarget.querySelector('.avatar__photo');
                if (avatar) {
                  avatar.style.transform = 'rotate(-5deg)';
                }
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.transform = 'translateY(0) scale(1)';
                e.currentTarget.style.boxShadow = '';
                e.currentTarget.style.borderColor = '';
                // Reset avatar rotation
                const avatar = e.currentTarget.querySelector('.avatar__photo');
                if (avatar) {
                  avatar.style.transform = 'rotate(0deg)';
                }
              }}
            >
              <div className="card__header text--center">
                <div className="avatar avatar--vertical">
                  {contributor.handle !== 'TBD' ? (
                    <img
                      className="avatar__photo avatar__photo--lg"
                      src={`https://github.com/${contributor.handle}.png`}
                      alt={contributor.name}
                      style={{ transition: 'transform 0.3s ease' }}
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
          <div className="card" style={{ padding: '20px', position: 'relative' }}>
            <div 
              style={{ 
                display: 'flex', 
                flexDirection: 'column',
                gap: '8px',
                fontSize: '14px',
                maxHeight: '550px',
                overflowY: 'auto',
                paddingRight: '8px'
              }}
              onScroll={handleScroll}
            >
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
                      backgroundColor: bgColor || 'var(--ifm-background-surface-color)', 
                      borderRadius: bgColor ? '8px' : '6px', 
                      fontWeight: bgColor ? 'bold' : '500',
                      boxShadow: bgColor ? '0 2px 4px rgba(0,0,0,0.1)' : 'none',
                      border: !bgColor ? '1px solid var(--ifm-color-emphasis-300)' : 'none',
                      transition: 'all 0.2s ease',
                      cursor: 'pointer'
                    }}
                    onMouseEnter={(e) => {
                      if (!bgColor) {
                        e.currentTarget.style.backgroundColor = 'var(--ifm-hover-overlay)';
                        e.currentTarget.style.transform = 'translateY(-1px)';
                        e.currentTarget.style.boxShadow = '0 2px 8px rgba(0,0,0,0.1)';
                      } else {
                        e.currentTarget.style.transform = 'translateY(-1px)';
                        e.currentTarget.style.boxShadow = '0 4px 12px rgba(0,0,0,0.15)';
                      }
                    }}
                    onMouseLeave={(e) => {
                      if (!bgColor) {
                        e.currentTarget.style.backgroundColor = 'var(--ifm-background-surface-color)';
                        e.currentTarget.style.transform = 'translateY(0)';
                        e.currentTarget.style.boxShadow = 'none';
                      } else {
                        e.currentTarget.style.transform = 'translateY(0)';
                        e.currentTarget.style.boxShadow = '0 2px 4px rgba(0,0,0,0.1)';
                      }
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
            {/* Simple scroll indicator - only show when not at bottom */}
            {showScrollIndicator && (
              <div style={{
                position: 'absolute',
                bottom: '20px',
                left: '50%',
                transform: 'translateX(-50%)',
                fontSize: '12px',
                color: 'var(--ifm-color-emphasis-600)',
                fontWeight: '500',
                pointerEvents: 'none',
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
                transition: 'opacity 0.3s ease'
              }}>
                Scroll for more ↓
              </div>
            )}
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-top--lg">
            <p>
              Thank you all for contributing! ❤️
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
                  fontSize: '24px',
                  color: '#666'
                }}>
                  ⭐
                </div>
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>Your Name Here</strong>
                <br />
                <small>Future Community Star</small>
              </div>
              <div style={{ fontSize: '14px', marginTop: '12px' }}>
                Want to be a Community All Star? Just start contributing on{' '}
                <Link href="https://github.com/block/goose">GitHub</Link>, helping others on{' '}
                <Link href="https://discord.gg/block-opensource">Discord</Link>, or share your 
                goose projects with the community! You can check out the{' '}
                <Link href="https://github.com/block/goose/blob/main/CONTRIBUTING.md">contributing guide</Link>{' '}
                for more tips.
              </div>
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
      <main>
        <UpcomingEventsSection />
        <CommunityAllStarsSection />
      </main>
    </Layout>
  );
}