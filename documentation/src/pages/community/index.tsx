import type { ReactNode } from "react";
import React from "react";
import Link from "@docusaurus/Link";
import Layout from "@theme/Layout";
import Heading from "@theme/Heading";

// Import community data
import communityConfig from "./data/config.json";
import april2025Data from "./data/april-2025.json";
import may2025Data from "./data/may-2025.json";
import june2025Data from "./data/june-2025.json";

// Create a data map for easy access
const communityDataMap = {
  "april-2025": april2025Data,
  "may-2025": may2025Data,
  "june-2025": june2025Data,
};

function UpcomingEventsSection() {
  return (
    <section className="w-full flex flex-col items-center gap-8 my-8">
      <div className="text-center">
        <Heading as="h1">Upcoming Events</Heading>
        <p>Join us for livestreams, workshops, and discussions about goose and open source projects.</p>
      </div>
      
      {/* Embedded Calendar */}
      <iframe
        src="https://calget.com/c/t7jszrie"
        className="w-full h-[600px] border-0 rounded-lg"
        title="Goose Community Calendar"
      />
      
      {/* Call to Action */}
      <p className="italic text-textStandard">
        Want to join us on a livestream or have ideas for future events? 
        Reach out to the team on <Link href="https://discord.gg/block-opensource">Discord</Link>.
      </p>
    </section>
  );
}

function CommunityAllStarsSection() {
  const [activeMonth, setActiveMonth] = React.useState(communityConfig.defaultMonth);
  const [showScrollIndicator, setShowScrollIndicator] = React.useState(true);
  
  const currentData = communityDataMap[activeMonth];

  const handleScroll = (e) => {
    const { scrollTop, scrollHeight, clientHeight } = e.target;
    const isAtBottom = scrollTop + clientHeight >= scrollHeight - 10; // 10px threshold
    setShowScrollIndicator(!isAtBottom);
  };

  return (
    <section className="w-full flex flex-col items-center gap-8 my-8">
      <div className="text-center">
        <Heading as="h1">Community All Stars</Heading>
        <p>Every month, we take a moment and celebrate the open source community. Here are the top contributors and community champions!</p>
      </div>
      
      {/* Month Tabs */}
      <div className="flex justify-center gap-2 flex-wrap">
        {communityConfig.availableMonths.map((month) => (
          <button 
            key={month.id}
            className="button button--primary"
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

      {/* Community Stars */}
      <div className="text-center">
        <Heading as="h3">⭐ Community Stars</Heading>
        <p className="text-sm text-textStandard">
          Top 5 Contributors from the open source community!
        </p>
      </div>
      
      <div className="flex justify-center">
        {currentData.communityStars.map((contributor, index) => (
          <StarsCard key={index} contributor={contributor} />
        ))}
      </div>
      
      {/* Team Stars */}
      <div className="text-center">
        <Heading as="h3">⭐ Team Stars</Heading>
        <p className="text-sm text-textStandard">
          Top 5 Contributors from all Block teams!
        </p>
      </div>
      
      <div className="flex justify-center">
        {currentData.teamStars.map((contributor, index) => (
          <StarsCard key={index} contributor={{...contributor, totalCount: currentData.teamStars.length}} />
        ))}
      </div>
      
      {/* Monthly Leaderboard */}
      <div className="text-center">
        <Heading as="h3">🏆 Monthly Leaderboard</Heading>
        <p className="text-sm text-textStandard">
          Rankings of all goose contributors getting loose this month!
        </p>
      </div>
      
      <div className="card w-full max-w-xl p-5 relative">
        <div 
          className="flex flex-col gap-2 text-sm max-h-[550px] overflow-y-auto pr-2"
          onScroll={handleScroll}
        >
          {currentData.leaderboard.map((contributor, index) => {
            const isTopContributor = index < 3; // Top 3 contributors

            const bgColor = index === 0 ? 'bg-yellow-400' :
              index === 1 ? 'bg-gray-300' :
              index === 2 ? 'bg-yellow-600' : null;
            
            return (
              <div 
                key={index}
                className={`flex items-center p-3 rounded-lg font-medium cursor-pointer transition-all duration-200 hover:-translate-y-0.5 ${
                  isTopContributor 
                    ? `${bgColor} font-bold shadow-md hover:shadow-lg` 
                    : 'bg-bgSubtle border border-borderStandard hover:bg-bgApp hover:shadow-md'
                }`}
              >
                {contributor.medal && (
                  <span className="mr-3 text-lg">
                    {contributor.medal}
                  </span>
                )}
                <span className={`mr-3 min-w-[30px] ${isTopContributor ? 'text-base text-black' : 'text-sm'}`}>
                  {contributor.rank}.
                </span>
                {contributor.handle !== 'TBD' ? (
                  <Link 
                    href={`https://github.com/${contributor.handle}`} 
                    className={`${isTopContributor ? 'text-black text-base' : 'text-inherit text-sm'}`}
                  >
                    @{contributor.handle}
                  </Link>
                ) : (
                  <span className="text-textSubtle italic">
                    @TBD
                  </span>
                )}
              </div>
            );
          })}
        </div>
        {/* Simple scroll indicator - only show when not at bottom */}
        {showScrollIndicator && (
          <div className="absolute bottom-5 inset-x-0 flex justify-center">
            <span className="w-fit text-xs bg-bgProminent p-2 rounded-full font-medium pointer-events-none flex items-center gap-1.5">
              Scroll for more ↓
            </span>
          </div>
        )}
      </div>
      
      <div className="text-center">
        <p>
          Thank you all for contributing! ❤️
        </p>
      </div>
      
      {/* Want to be featured section */}
      <div className="text-center">
        <Heading as="h2">Want to be featured?</Heading>
      </div>
      
      <div className="card max-w-xl">
        <div className="card__header text-center">
          <div className="avatar avatar--vertical">
            <div className="w-16 h-16 rounded-full bg-blue-400 flex items-center justify-center text-2xl text-blue-500">
              ⭐
            </div>
          </div>
        </div>
        <div className="card__body text--center">
          <div className="mb-4">
            <strong>Your Name Here</strong>
            <br />
            <small>Future Community Star</small>
          </div>
          <div className="text-sm">
            Want to be a Community All Star? Just start contributing on{' '}
            <Link href="https://github.com/block/goose">GitHub</Link>, helping others on{' '}
            <Link href="https://discord.gg/block-opensource">Discord</Link>, or share your 
            goose projects with the community! You can check out the{' '}
            <Link href="https://github.com/block/goose/blob/main/CONTRIBUTING.md">contributing guide</Link>{' '}
            for more tips.
          </div>
        </div>
      </div>
    </section>
  );
}

export function StarsCard({contributor}): ReactNode {
  return (
    <div className={`col ${contributor.totalCount <= 3 ? 'col--4' : 'col--2'} mb-8`}>
      <div 
        className="h-full border-2 border-borderSubtle rounded-2xl cursor-pointer hover:shadow-xl hover:border-[var(--ifm-color-primary-dark)]"
      >
        <div className="card__header text-center">
          <div className="avatar avatar--vertical">
            {contributor.avatarUrl ? (
              <img
                className="avatar__photo avatar__photo--lg"
                src={contributor.avatarUrl}
                alt={contributor.name}
              />
            ) : contributor.handle !== 'TBD' ? (
              <img
                className="avatar__photo avatar__photo--lg"
                src={`https://github.com/${contributor.handle}.png`}
                alt={contributor.name}
              />
            ) : (
              <div className="w-16 h-16 rounded-full bg-gray-200 flex items-center justify-center text-xl text-textSubtle">
                ?
              </div>
            )}
          </div>
        </div>
        <div className="card__body text-center">
          <div className="mb-2">
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
  );
};

export default function Community(): ReactNode {
  return (
    <Layout 
      title="Community" 
      description="Join the Goose community - connect with developers, contribute to the project, and help shape the future of AI-powered development tools."
    >
      <main className="container">
        <UpcomingEventsSection />
        <CommunityAllStarsSection />
      </main>
    </Layout>
  );
}