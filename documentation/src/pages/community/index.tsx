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
import july2025Data from "./data/july-2025.json";
import august2025Data from "./data/august-2025.json";
import september2025Data from "./data/september-2025.json";
import october2025Data from "./data/october-2025.json";
import november2025Data from "./data/november-2025.json";
import communityContentData from "./data/community-content.json";

// Create a data map for easy access
const communityDataMap = {
  "april-2025": april2025Data,
  "may-2025": may2025Data,
  "june-2025": june2025Data,
  "july-2025": july2025Data,
  "august-2025": august2025Data,
  "september-2025": september2025Data,
  "october-2025": october2025Data,
  "november-2025": november2025Data,
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
        Reach out to the team on <Link href="https://discord.gg/goose-oss">Discord</Link>.
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
      
      <div className="flex flex-wrap justify-center gap-4 w-full px-4">
        {currentData.communityStars.map((contributor, index) => (
          <StarsCard key={index} contributor={contributor} />
        ))}
      </div>
      
      {/* Team Stars - only show if there are team stars */}
      {currentData.teamStars.length > 0 && (
        <>
          <div className="text-center">
            <Heading as="h3">⭐ Team Stars</Heading>
            <p className="text-sm text-textStandard">
              Top 5 Contributors from all Block teams!
            </p>
          </div>
          
          <div className="flex flex-wrap justify-center gap-4 w-full px-4">
            {currentData.teamStars.map((contributor, index) => (
              <StarsCard key={index} contributor={{...contributor, totalCount: currentData.teamStars.length}} />
            ))}
          </div>
        </>
      )}
      
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
            <Link href="https://discord.gg/goose-oss">Discord</Link>, or share your 
            goose projects with the community! You can check out the{' '}
            <Link href="https://github.com/block/goose/blob/main/CONTRIBUTING.md">contributing guide</Link>{' '}
            for more tips.
          </div>
        </div>
      </div>
    </section>
  );
}

function CommunityContentSpotlightSection() {
  const [contentFilter, setContentFilter] = React.useState('all');
  const [showScrollIndicator, setShowScrollIndicator] = React.useState(true);
  
  const filteredSubmissions = React.useMemo(() => {
    if (contentFilter === 'all') return communityContentData.submissions;
    if (contentFilter === 'hacktoberfest') {
      return communityContentData.submissions.filter(content => 
        content.hacktoberfest || content.tags?.includes('hacktoberfest')
      );
    }
    return communityContentData.submissions.filter(content => content.type === contentFilter);
  }, [contentFilter]);

  const handleScroll = (e: React.UIEvent<HTMLDivElement>) => {
    const { scrollTop, scrollHeight, clientHeight } = (e.target as HTMLDivElement);
    const isAtBottom = scrollTop + clientHeight >= scrollHeight - 10; // 10px threshold
    setShowScrollIndicator(!isAtBottom);
  };

  return (
    <section className="w-full flex flex-col items-center gap-8 my-8">
      <div className="text-center">
        <Heading as="h1">{communityContentData.title}</Heading>
        <p>{communityContentData.description}</p>
      </div>
      
      {/* Filter Tabs */}
      <div className="flex justify-center gap-2 flex-wrap">
        {[
          { id: 'all', label: 'All Content' },
          { id: 'hacktoberfest', label: '🎃 Hacktoberfest 2025' },
          { id: 'blog', label: '📝 Blog Posts' },
          { id: 'video', label: '🎥 Videos' }
        ].map((filter) => (
          <button 
            key={filter.id}
            className="button button--secondary"
            onClick={() => setContentFilter(filter.id)}
            style={contentFilter === filter.id ? {
              backgroundColor: 'var(--ifm-color-primary)',
              color: 'white',
              border: '2px solid var(--ifm-color-primary-dark)'
            } : {}}
          >
            {filter.label}
          </button>
        ))}
      </div>
      
      {/* Content Grid */}
      <div className="w-full max-w-6xl relative">
        <div 
          className="max-h-[800px] overflow-y-auto pr-2"
          onScroll={handleScroll}
        >
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {/* Persistent Hacktoberfest CTA Card */}
            <HacktoberfestCTACard />
            
            {filteredSubmissions.map((content) => (
              <ContentCard key={content.url} content={content} />
            ))}
          </div>
          
          {filteredSubmissions.length === 0 && (
            <div className="text-center py-8">
              <p className="text-textSubtle">No content found for this filter.</p>
            </div>
          )}
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
    </section>
  );
}

function HacktoberfestCTACard(): ReactNode {
  return (
    <div className="card h-full transition-all duration-200 hover:shadow-lg hover:-translate-y-1 bg-gradient-to-br from-orange-100 to-purple-100 border-2 border-orange-300">
      {/* Thumbnail placeholder */}
      <div className="card__image relative">
        <div className="w-full h-48 bg-gradient-to-br from-orange-200 to-purple-200 flex items-center justify-center">
          <span className="text-6xl">🎃</span>
        </div>
        <div className="absolute top-2 left-2 bg-orange-500 text-white px-2 py-1 rounded-full text-xs font-bold flex items-center gap-1">
          🎃 Hacktoberfest
        </div>
      </div>
      
      {/* Content */}
      <div className="card__body">
        {/* CTA Button as Title */}
        <div className="mb-3">
          <Link 
            href={communityContentData.submissionUrl}
            className="button button--primary button--block button--lg"
            target="_blank"
            rel="noopener noreferrer"
          >
            🚀 Submit Your Content!
          </Link>
        </div>
        
        {/* Description */}
        <div className="text-sm text-textSubtle mb-2">
          <p>Share your goose blog posts or videos with the community.</p>
        </div>
        
        <p className="text-xs text-textSubtle text-center">
          Must be hosted on your own website
        </p>
      </div>
    </div>
  );
}

function ContentCard({ content }): ReactNode {
  const getTypeIcon = (type: string) => {
    switch (type) {
      case 'blog': return '📝';
      case 'video': return '🎥';
      case 'tutorial': return '📚';
      case 'case-study': return '📊';
      default: return '📄';
    }
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('en-US', { 
      month: 'short', 
      day: 'numeric' 
    });
  };

  return (
    <div className="card h-full transition-all duration-200 hover:shadow-lg hover:-translate-y-1">
      {/* Thumbnail */}
      <div className="card__image relative">
        <img
          src={content.thumbnail || 'https://images.unsplash.com/photo-1516321318423-f06f85e504b3?w=400&h=225&fit=crop&crop=entropy&auto=format'}
          alt={content.title}
          className="w-full h-48 object-cover"
          loading="lazy"
        />
      </div>
      
      {/* Content */}
      <div className="card__body">
        <div className="flex items-start gap-2 mb-2">
          <span className="text-lg">{getTypeIcon(content.type)}</span>
          <h3 className="text-lg font-semibold line-clamp-2 flex-1">
            <Link href={content.url} className="text-inherit hover:text-primary">
              {content.title}
            </Link>
          </h3>
        </div>
        
        {/* Author and Date */}
        <div className="flex items-center justify-between text-sm text-textSubtle mb-3">
          <div className="flex items-center gap-2">
            <img
              src={`https://github.com/${content.author.handle}.png`}
              alt={content.author.name}
              className="w-6 h-6 rounded-full"
            />
            <Link href={`https://github.com/${content.author.handle}`} className="hover:text-primary">
              @{content.author.handle}
            </Link>
          </div>
          <span>📅 {formatDate(content.submittedDate)}</span>
        </div>
        

      </div>
      

    </div>
  );
}

export function StarsCard({contributor}): ReactNode {
  return (
    <div className="w-full sm:w-[calc(50%-0.5rem)] md:w-[calc(33.333%-0.67rem)] lg:w-[calc(20%-0.8rem)] max-w-[280px]">
      <div 
        className="h-full border-2 border-borderSubtle rounded-2xl cursor-pointer hover:shadow-xl hover:border-[var(--ifm-color-primary-dark)] transition-all"
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
      description="Join the goose community - connect with developers, contribute to the project, and help shape the future of AI-powered development tools."
    >
      <main className="container">
        <CommunityAllStarsSection />
        <CommunityContentSpotlightSection />
        <UpcomingEventsSection />
      </main>
    </Layout>
  );
}
