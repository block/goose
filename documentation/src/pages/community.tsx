import type { ReactNode } from "react";
import React from "react";
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
  const [activeMonth, setActiveMonth] = React.useState('May 2025');
  
  // May 2025 data (current)
  const may2025Data = {
    featuredContributors: [
      { name: 'Rizel Scarlett', handle: 'blackgirlbytes', role: 'Developer Advocate', avatar: 'https://github.com/blackgirlbytes.png' },
      { name: 'Zane Squires', handle: 'zanesq', role: 'Senior Software Engineer', avatar: 'https://github.com/zanesq.png' },
      { name: 'The-Best-Codes', handle: 'The-Best-Codes', role: 'Open Source Developer', avatar: 'https://github.com/The-Best-Codes.png' },
      { name: 'Michael Neale', handle: 'michaelneale', role: 'Principal Engineer', avatar: 'https://github.com/michaelneale.png' },
      { name: 'Patrick Reis', handle: 'patrickReiis', role: 'Community Contributor', avatar: 'https://github.com/patrickReiis.png' }
    ],
    risingStars: [
      { name: 'Angie Jones', handle: 'angiejones', role: 'Senior Developer', avatar: 'https://github.com/angiejones.png' },
      { name: 'Sven-Hendrik Haase', handle: 'svenstaro', role: 'Open Source Maintainer', avatar: 'https://github.com/svenstaro.png' },
      { name: 'faces-of-eth', handle: 'faces-of-eth', role: 'Community Developer', avatar: 'https://github.com/faces-of-eth.png' },
      { name: 'Wes Billman', handle: 'wesbillman', role: 'Software Engineer', avatar: 'https://github.com/wesbillman.png' },
      { name: 'Antonio Cheong', handle: 'acheong08', role: 'AI Developer', avatar: 'https://github.com/acheong08.png' }
    ],
    leaderboard: [
      { name: 'blackgirlbytes', rank: 1, medal: '🥇', bgColor: '#FFD700' },
      { name: 'zanesq', rank: 2, medal: '🥈', bgColor: '#C0C0C0' },
      { name: 'michaelneale', rank: 3, medal: '🥉', bgColor: '#CD7F32' },
      { name: 'angiejones', rank: 4 },
      { name: 'Kvadratni', rank: 5 },
      { name: 'lifeizhou-ap', rank: 6 },
      { name: 'dianed-square', rank: 7 },
      { name: 'yingjiehe-xyz', rank: 8 },
      { name: 'salman1993', rank: 9 },
      { name: 'ahau-square', rank: 10 },
      { name: 'iandouglas', rank: 11 },
      { name: 'emma-squared', rank: 12 },
      { name: 'dbraduan', rank: 13 },
      { name: 'lily-de', rank: 14 },
      { name: 'alexhancock', rank: 15 },
      { name: 'EbonyLouis', rank: 16 },
      { name: 'wendytang', rank: 17 },
      { name: 'The-Best-Codes', rank: 18 },
      { name: 'opdich', rank: 19 },
      { name: 'agiuliano-square', rank: 20 },
      { name: 'patrickReiis', rank: 21 },
      { name: 'kalvinnchau', rank: 22 },
      { name: 'acekyd', rank: 23 },
      { name: 'nahiyankhan', rank: 24 },
      { name: 'taniashiba', rank: 25 },
      { name: 'JohnMAustin78', rank: 26 },
      { name: 'sheagcraig', rank: 27 },
      { name: 'alicehau', rank: 28 },
      { name: 'bwalding', rank: 29 },
      { name: 'jamadeo', rank: 30 },
      { name: 'rockwotj', rank: 31 },
      { name: 'danielzayas', rank: 32 },
      { name: 'svenstaro', rank: 33 },
      { name: 'adaug', rank: 34 },
      { name: 'loganmoseley', rank: 35 },
      { name: 'tiborvass', rank: 36 },
      { name: 'xuv', rank: 37 },
      { name: 'anilmuppalla', rank: 38 },
      { name: 'spencrmartin', rank: 39 },
      { name: 'gknoblauch', rank: 40 },
      { name: 'acheong08', rank: 41 },
      { name: 'faces-of-eth', rank: 42 },
      { name: 'wesbillman', rank: 43 }
    ]
  };
  
  // April 2025 data (placeholder)
  const april2025Data = {
    featuredContributors: [
      { name: 'Coming Soon', handle: 'TBD', role: 'To Be Announced', avatar: null },
      { name: 'Coming Soon', handle: 'TBD', role: 'To Be Announced', avatar: null },
      { name: 'Coming Soon', handle: 'TBD', role: 'To Be Announced', avatar: null },
      { name: 'Coming Soon', handle: 'TBD', role: 'To Be Announced', avatar: null },
      { name: 'Coming Soon', handle: 'TBD', role: 'To Be Announced', avatar: null }
    ],
    risingStars: [
      { name: 'Coming Soon', handle: 'TBD', role: 'To Be Announced', avatar: null },
      { name: 'Coming Soon', handle: 'TBD', role: 'To Be Announced', avatar: null },
      { name: 'Coming Soon', handle: 'TBD', role: 'To Be Announced', avatar: null },
      { name: 'Coming Soon', handle: 'TBD', role: 'To Be Announced', avatar: null },
      { name: 'Coming Soon', handle: 'TBD', role: 'To Be Announced', avatar: null }
    ],
    leaderboard: Array.from({ length: 43 }, (_, i) => ({
      name: 'TBD',
      rank: i + 1,
      medal: i === 0 ? '🥇' : i === 1 ? '🥈' : i === 2 ? '🥉' : null,
      bgColor: i === 0 ? '#FFD700' : i === 1 ? '#C0C0C0' : i === 2 ? '#CD7F32' : null
    }))
  };
  
  const currentData = activeMonth === 'May 2025' ? may2025Data : april2025Data;

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
              <button 
                className={`button ${activeMonth === 'April 2025' ? 'button--primary' : 'button--outline button--primary'}`}
                onClick={() => setActiveMonth('April 2025')}
              >
                April 2025
              </button>
              <button 
                className={`button ${activeMonth === 'May 2025' ? 'button--primary' : 'button--outline button--primary'}`}
                onClick={() => setActiveMonth('May 2025')}
              >
                May 2025
              </button>
            </div>
          </div>
        </div>
      </div>
      
      {/* First Group of 5 */}
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-bottom--md">
            <Heading as="h3">Featured Contributors</Heading>
          </div>
        </div>
      </div>
      
      <div className="row">
        {currentData.featuredContributors.map((contributor, index) => (
          <div key={index} className={`col col--2 ${index === 0 ? 'col--offset-1' : ''} margin-bottom--lg`}>
            <div className="card">
              <div className="card__header text--center">
                <div className="avatar avatar--vertical">
                  {contributor.avatar ? (
                    <img
                      className="avatar__photo avatar__photo--lg"
                      src={contributor.avatar}
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
                    {contributor.avatar ? (
                      <Link href={`https://github.com/${contributor.handle}`}>
                        {contributor.name}
                      </Link>
                    ) : (
                      contributor.name
                    )}
                  </strong>
                  <br />
                  <small>@{contributor.handle}</small>
                  <br />
                  <small>{contributor.role}</small>
                </div>
              </div>
            </div>
          </div>
        ))}
      </div>
      
      {/* Second Group of 5 */}
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-bottom--md">
            <Heading as="h3">Rising Stars</Heading>
          </div>
        </div>
      </div>
      
      <div className="row">
        {currentData.risingStars.map((contributor, index) => (
          <div key={index} className={`col col--2 ${index === 0 ? 'col--offset-1' : ''} margin-bottom--lg`}>
            <div className="card">
              <div className="card__header text--center">
                <div className="avatar avatar--vertical">
                  {contributor.avatar ? (
                    <img
                      className="avatar__photo avatar__photo--lg"
                      src={contributor.avatar}
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
                    {contributor.avatar ? (
                      <Link href={`https://github.com/${contributor.handle}`}>
                        {contributor.name}
                      </Link>
                    ) : (
                      contributor.name
                    )}
                  </strong>
                  <br />
                  <small>@{contributor.handle}</small>
                  <br />
                  <small>{contributor.role}</small>
                </div>
              </div>
            </div>
          </div>
        ))}
      </div>
      
      {/* Third Group - All Contributors Leaderboard */}
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-bottom--md">
            <Heading as="h3">🏆 Contributors Leaderboard</Heading>
            <p style={{ fontSize: '14px', color: 'var(--ifm-color-emphasis-600)' }}>
              All 43 amazing contributors who make Goose possible!
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
              {currentData.leaderboard.map((contributor, index) => (
                <div 
                  key={index}
                  style={{ 
                    display: 'flex', 
                    alignItems: 'center', 
                    padding: contributor.bgColor ? '12px' : '10px', 
                    backgroundColor: contributor.bgColor || '#f8f9fa', 
                    borderRadius: contributor.bgColor ? '8px' : '6px', 
                    fontWeight: contributor.bgColor ? 'bold' : '500',
                    boxShadow: contributor.bgColor ? '0 2px 4px rgba(0,0,0,0.1)' : 'none',
                    borderLeft: !contributor.bgColor ? '4px solid #e9ecef' : 'none'
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
                    fontSize: contributor.bgColor ? '16px' : '14px' 
                  }}>
                    {contributor.rank}.
                  </span>
                  {contributor.name !== 'TBD' ? (
                    <Link 
                      href={`https://github.com/${contributor.name}`} 
                      style={{ 
                        color: contributor.bgColor ? '#000' : 'inherit',
                        fontSize: contributor.bgColor ? '16px' : '14px'
                      }}
                    >
                      @{contributor.name}
                    </Link>
                  ) : (
                    <span style={{ color: '#999', fontStyle: 'italic' }}>
                      @TBD
                    </span>
                  )}
                </div>
              ))}
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