import React, { useState } from 'react';
import { motion } from 'framer-motion';
import { 
  Package, 
  Download, 
  Star,
  Search,
  TrendingUp,
  Clock
} from 'lucide-react';
import GooseAppImage from '../../assets/GooseApp.png';
import AppsImage from '../../assets/Apps.png';
import G2Image from '../../assets/g2.png';
import PeepsImage from '../../assets/peeps.png';
import ExtensionsImage from '../../assets/extensions.png';
import SellersImage from '../../assets/sellers.png';

interface MarketplaceItem {
  id: string;
  name: string;
  description: string;
  type: 'recipe' | 'extension' | 'template';
  author: string;
  downloads: number;
  rating: number;
  lastUpdated: Date;
  tags: string[];
  icon?: string;
  featured?: boolean;
  fullWidth?: boolean;
  hero?: boolean;
  gradient?: string;
  tagline?: string;
  image?: string;
}

// Mock data - will be replaced with real data later
const mockItems: MarketplaceItem[] = [
  // 1. First hero card
  {
    id: 'hero-1',
    name: 'Goose iOS',
    description: 'Your AI development companion on the go. Access powerful coding assistance, run recipes, and manage your projects from anywhere. Available now on the App Store.',
    type: 'extension',
    author: 'Block Team',
    downloads: 25680,
    rating: 5.0,
    lastUpdated: new Date('2024-01-29'),
    tags: ['ios', 'mobile', 'app', 'hero'],
    featured: true,
    hero: true,
    gradient: 'from-indigo-600 via-purple-600 to-pink-600',
    tagline: '📱 Now on iOS',
  },
  // 2. Two featured cards
  {
    id: 'featured-1',
    name: 'AI Code Assistant Pro',
    description: 'Transform your development workflow with AI-powered code generation, intelligent refactoring, and real-time code reviews. Supports 50+ languages and frameworks.',
    type: 'extension',
    author: 'Block Team',
    downloads: 15420,
    rating: 4.9,
    lastUpdated: new Date('2024-01-28'),
    tags: ['ai', 'code-generation', 'productivity', 'featured'],
    featured: true,
    fullWidth: true,
    gradient: 'from-purple-500 via-pink-500 to-red-500',
    tagline: '🚀 Most Popular Extension',
    image: 'apps',
  },
  {
    id: 'featured-2',
    name: 'Full-Stack Deployment Suite',
    description: 'Deploy anywhere in seconds. One-click deployment to AWS, Azure, GCP, Vercel, and more. Includes CI/CD pipelines, monitoring, and automatic rollbacks.',
    type: 'template',
    author: 'Block Team',
    downloads: 12890,
    rating: 4.9,
    lastUpdated: new Date('2024-01-27'),
    tags: ['deployment', 'devops', 'cloud', 'featured'],
    featured: true,
    fullWidth: true,
    gradient: 'from-blue-500 via-cyan-500 to-teal-500',
    tagline: '⚡ Deploy in Seconds',
    image: 'g2',
  },
  // 3. Second hero card (full width)
  {
    id: 'hero-2',
    name: 'Enterprise Solutions',
    description: 'Scale your development with enterprise-grade tools, advanced security features, and dedicated support. Built for teams that demand the best.',
    type: 'extension',
    author: 'Block Team',
    downloads: 18500,
    rating: 5.0,
    lastUpdated: new Date('2024-01-30'),
    tags: ['enterprise', 'security', 'teams', 'hero'],
    featured: true,
    hero: true,
    gradient: 'from-blue-600 to-blue-600',
    tagline: '🏢 For Teams',
    image: 'peeps',
  },
  // 4. Second set of featured cards
  {
    id: 'featured-3',
    name: 'Cloud Infrastructure Manager',
    description: 'Manage your cloud infrastructure across AWS, Azure, and GCP from a single interface. Automated provisioning, monitoring, and cost optimization.',
    type: 'extension',
    author: 'Block Team',
    downloads: 9850,
    rating: 4.8,
    lastUpdated: new Date('2024-01-26'),
    tags: ['cloud', 'infrastructure', 'devops', 'featured'],
    featured: true,
    fullWidth: true,
    gradient: 'from-green-500 via-emerald-500 to-teal-500',
    tagline: '☁️ Cloud Native',
    image: 'extensions',
  },
  {
    id: 'featured-4',
    name: 'API Testing Suite',
    description: 'Comprehensive API testing with automated test generation, load testing, and real-time monitoring. Supports REST, GraphQL, and WebSocket APIs.',
    type: 'template',
    author: 'Block Team',
    downloads: 11200,
    rating: 4.9,
    lastUpdated: new Date('2024-01-28'),
    tags: ['testing', 'api', 'automation', 'featured'],
    featured: true,
    fullWidth: true,
    gradient: 'from-orange-500 via-red-500 to-pink-500',
    tagline: '🧪 Test Everything',
    image: 'sellers',
  },
  // 5. Rest of the regular tiles
  {
    id: '1',
    name: 'Python Code Review',
    description: 'Automated code review assistant for Python projects',
    type: 'recipe',
    author: 'Block Team',
    downloads: 1250,
    rating: 4.8,
    lastUpdated: new Date('2024-01-15'),
    tags: ['python', 'code-review', 'quality'],
  },
  {
    id: '2',
    name: 'React Component Generator',
    description: 'Generate React components with TypeScript and tests',
    type: 'recipe',
    author: 'Community',
    downloads: 890,
    rating: 4.5,
    lastUpdated: new Date('2024-01-10'),
    tags: ['react', 'typescript', 'components'],
  },
  {
    id: '3',
    name: 'API Documentation Writer',
    description: 'Automatically generate API documentation from code',
    type: 'recipe',
    author: 'Block Team',
    downloads: 2100,
    rating: 4.9,
    lastUpdated: new Date('2024-01-20'),
    tags: ['documentation', 'api', 'openapi'],
  },
  {
    id: '4',
    name: 'Git Workflow Helper',
    description: 'Streamline your git workflow with smart commit messages and branch management',
    type: 'recipe',
    author: 'Community',
    downloads: 1580,
    rating: 4.7,
    lastUpdated: new Date('2024-01-18'),
    tags: ['git', 'workflow', 'automation'],
  },
  {
    id: '5',
    name: 'Database Schema Designer',
    description: 'Design and generate database schemas with best practices',
    type: 'extension',
    author: 'Block Team',
    downloads: 2340,
    rating: 4.9,
    lastUpdated: new Date('2024-01-22'),
    tags: ['database', 'schema', 'sql'],
  },
  {
    id: '6',
    name: 'Test Suite Generator',
    description: 'Automatically generate comprehensive test suites for your code',
    type: 'recipe',
    author: 'Community',
    downloads: 1120,
    rating: 4.6,
    lastUpdated: new Date('2024-01-12'),
    tags: ['testing', 'automation', 'quality'],
  },
  {
    id: '7',
    name: 'REST API Builder',
    description: 'Scaffold complete REST APIs with authentication and validation',
    type: 'template',
    author: 'Block Team',
    downloads: 3200,
    rating: 4.8,
    lastUpdated: new Date('2024-01-25'),
    tags: ['api', 'rest', 'backend'],
  },
  {
    id: '8',
    name: 'Docker Compose Generator',
    description: 'Create optimized Docker Compose configurations for your stack',
    type: 'recipe',
    author: 'Community',
    downloads: 1890,
    rating: 4.7,
    lastUpdated: new Date('2024-01-19'),
    tags: ['docker', 'devops', 'containers'],
  },
  {
    id: '9',
    name: 'GraphQL Schema Builder',
    description: 'Design and implement GraphQL schemas with resolvers',
    type: 'extension',
    author: 'Block Team',
    downloads: 1450,
    rating: 4.6,
    lastUpdated: new Date('2024-01-16'),
    tags: ['graphql', 'api', 'schema'],
  },
  {
    id: '10',
    name: 'CI/CD Pipeline Creator',
    description: 'Generate CI/CD pipelines for GitHub Actions, GitLab, and more',
    type: 'recipe',
    author: 'Community',
    downloads: 2670,
    rating: 4.9,
    lastUpdated: new Date('2024-01-23'),
    tags: ['ci-cd', 'devops', 'automation'],
  },
  {
    id: '11',
    name: 'Microservices Template',
    description: 'Complete microservices architecture with service mesh',
    type: 'template',
    author: 'Block Team',
    downloads: 2890,
    rating: 4.8,
    lastUpdated: new Date('2024-01-21'),
    tags: ['microservices', 'architecture', 'kubernetes'],
  },
  {
    id: '12',
    name: 'Code Refactoring Assistant',
    description: 'Intelligent code refactoring suggestions and automated improvements',
    type: 'recipe',
    author: 'Community',
    downloads: 1340,
    rating: 4.5,
    lastUpdated: new Date('2024-01-14'),
    tags: ['refactoring', 'code-quality', 'optimization'],
  },
  {
    id: '13',
    name: 'Security Audit Tool',
    description: 'Scan your codebase for security vulnerabilities and best practices',
    type: 'extension',
    author: 'Block Team',
    downloads: 3450,
    rating: 4.9,
    lastUpdated: new Date('2024-01-24'),
    tags: ['security', 'audit', 'vulnerability'],
  },
  {
    id: '14',
    name: 'TypeScript Migrator',
    description: 'Migrate JavaScript projects to TypeScript with type inference',
    type: 'recipe',
    author: 'Community',
    downloads: 1670,
    rating: 4.7,
    lastUpdated: new Date('2024-01-17'),
    tags: ['typescript', 'migration', 'javascript'],
  },
  {
    id: '15',
    name: 'Mobile App Starter',
    description: 'React Native starter template with navigation and state management',
    type: 'template',
    author: 'Block Team',
    downloads: 2120,
    rating: 4.6,
    lastUpdated: new Date('2024-01-13'),
    tags: ['react-native', 'mobile', 'starter'],
  },
  {
    id: '16',
    name: 'Performance Optimizer',
    description: 'Analyze and optimize application performance bottlenecks',
    type: 'recipe',
    author: 'Community',
    downloads: 1980,
    rating: 4.8,
    lastUpdated: new Date('2024-01-20'),
    tags: ['performance', 'optimization', 'profiling'],
  },
  {
    id: '17',
    name: 'Kubernetes Deployer',
    description: 'Generate Kubernetes manifests and deployment strategies',
    type: 'extension',
    author: 'Block Team',
    downloads: 2560,
    rating: 4.7,
    lastUpdated: new Date('2024-01-22'),
    tags: ['kubernetes', 'deployment', 'devops'],
  },
  {
    id: '18',
    name: 'Accessibility Checker',
    description: 'Ensure your web apps meet WCAG accessibility standards',
    type: 'recipe',
    author: 'Community',
    downloads: 1230,
    rating: 4.6,
    lastUpdated: new Date('2024-01-11'),
    tags: ['accessibility', 'a11y', 'wcag'],
  },
  {
    id: '19',
    name: 'Serverless Framework',
    description: 'Deploy serverless functions to AWS Lambda, Azure, and GCP',
    type: 'template',
    author: 'Block Team',
    downloads: 2780,
    rating: 4.8,
    lastUpdated: new Date('2024-01-26'),
    tags: ['serverless', 'lambda', 'cloud'],
  },
  {
    id: '20',
    name: 'E2E Test Automation',
    description: 'Playwright and Cypress test automation for web applications',
    type: 'recipe',
    author: 'Community',
    downloads: 1560,
    rating: 4.7,
    lastUpdated: new Date('2024-01-19'),
    tags: ['testing', 'e2e', 'automation'],
  },
  {
    id: '21',
    name: 'Monorepo Manager',
    description: 'Manage monorepos with Nx, Turborepo, or Lerna',
    type: 'extension',
    author: 'Block Team',
    downloads: 2140,
    rating: 4.6,
    lastUpdated: new Date('2024-01-21'),
    tags: ['monorepo', 'workspace', 'tooling'],
  },
  {
    id: '22',
    name: 'CSS Framework Generator',
    description: 'Generate custom Tailwind, Bootstrap, or Material UI themes',
    type: 'recipe',
    author: 'Community',
    downloads: 1890,
    rating: 4.5,
    lastUpdated: new Date('2024-01-15'),
    tags: ['css', 'styling', 'themes'],
  },
  {
    id: '23',
    name: 'Data Migration Tool',
    description: 'Migrate data between databases with validation and rollback',
    type: 'extension',
    author: 'Block Team',
    downloads: 1720,
    rating: 4.8,
    lastUpdated: new Date('2024-01-23'),
    tags: ['database', 'migration', 'etl'],
  },
  {
    id: '24',
    name: 'Logging & Monitoring',
    description: 'Integrate structured logging and monitoring solutions',
    type: 'recipe',
    author: 'Community',
    downloads: 2340,
    rating: 4.7,
    lastUpdated: new Date('2024-01-20'),
    tags: ['logging', 'monitoring', 'observability'],
  },
  {
    id: '25',
    name: 'OAuth Provider Setup',
    description: 'Implement OAuth 2.0 authentication with multiple providers',
    type: 'template',
    author: 'Block Team',
    downloads: 3120,
    rating: 4.9,
    lastUpdated: new Date('2024-01-25'),
    tags: ['auth', 'oauth', 'security'],
  },
  {
    id: '26',
    name: 'Markdown Documentation',
    description: 'Generate beautiful documentation sites from markdown files',
    type: 'recipe',
    author: 'Community',
    downloads: 1450,
    rating: 4.6,
    lastUpdated: new Date('2024-01-16'),
    tags: ['documentation', 'markdown', 'static-site'],
  },
  {
    id: '27',
    name: 'WebSocket Server',
    description: 'Real-time WebSocket server with Socket.io or native WS',
    type: 'template',
    author: 'Block Team',
    downloads: 1980,
    rating: 4.7,
    lastUpdated: new Date('2024-01-22'),
    tags: ['websocket', 'realtime', 'socket.io'],
  },
  {
    id: '28',
    name: 'Code Formatter',
    description: 'Auto-format code with Prettier, ESLint, and custom rules',
    type: 'recipe',
    author: 'Community',
    downloads: 2890,
    rating: 4.8,
    lastUpdated: new Date('2024-01-24'),
    tags: ['formatting', 'linting', 'code-quality'],
  },
  {
    id: '29',
    name: 'Feature Flag Manager',
    description: 'Implement feature flags with LaunchDarkly or custom solution',
    type: 'extension',
    author: 'Block Team',
    downloads: 1670,
    rating: 4.6,
    lastUpdated: new Date('2024-01-18'),
    tags: ['feature-flags', 'deployment', 'configuration'],
  },
  {
    id: '30',
    name: 'Email Template Builder',
    description: 'Create responsive email templates with MJML or React Email',
    type: 'recipe',
    author: 'Community',
    downloads: 1340,
    rating: 4.5,
    lastUpdated: new Date('2024-01-14'),
    tags: ['email', 'templates', 'mjml'],
  },
  {
    id: '31',
    name: 'Payment Integration',
    description: 'Integrate Stripe, PayPal, or Square payment processing',
    type: 'template',
    author: 'Block Team',
    downloads: 3450,
    rating: 4.9,
    lastUpdated: new Date('2024-01-27'),
    tags: ['payments', 'stripe', 'ecommerce'],
  },
  {
    id: '32',
    name: 'Caching Strategy',
    description: 'Implement Redis, Memcached, or in-memory caching',
    type: 'recipe',
    author: 'Community',
    downloads: 2120,
    rating: 4.7,
    lastUpdated: new Date('2024-01-21'),
    tags: ['caching', 'redis', 'performance'],
  },
  {
    id: '33',
    name: 'Internationalization',
    description: 'Add i18n support with react-intl or next-i18next',
    type: 'extension',
    author: 'Block Team',
    downloads: 1890,
    rating: 4.6,
    lastUpdated: new Date('2024-01-19'),
    tags: ['i18n', 'localization', 'translation'],
  },
  {
    id: '34',
    name: 'Error Tracking',
    description: 'Integrate Sentry, Rollbar, or custom error tracking',
    type: 'recipe',
    author: 'Community',
    downloads: 2560,
    rating: 4.8,
    lastUpdated: new Date('2024-01-23'),
    tags: ['errors', 'monitoring', 'sentry'],
  },
  {
    id: '35',
    name: 'Admin Dashboard',
    description: 'Full-featured admin dashboard with charts and tables',
    type: 'template',
    author: 'Block Team',
    downloads: 3890,
    rating: 4.9,
    lastUpdated: new Date('2024-01-26'),
    tags: ['admin', 'dashboard', 'ui'],
  },
  {
    id: '36',
    name: 'Rate Limiter',
    description: 'API rate limiting with Redis or in-memory storage',
    type: 'recipe',
    author: 'Community',
    downloads: 1780,
    rating: 4.7,
    lastUpdated: new Date('2024-01-17'),
    tags: ['rate-limiting', 'api', 'security'],
  },
  {
    id: '37',
    name: 'Search Engine',
    description: 'Integrate Elasticsearch, Algolia, or MeiliSearch',
    type: 'extension',
    author: 'Block Team',
    downloads: 2340,
    rating: 4.8,
    lastUpdated: new Date('2024-01-24'),
    tags: ['search', 'elasticsearch', 'indexing'],
  },
  {
    id: '38',
    name: 'Image Optimizer',
    description: 'Optimize and transform images with Sharp or Cloudinary',
    type: 'recipe',
    author: 'Community',
    downloads: 1560,
    rating: 4.6,
    lastUpdated: new Date('2024-01-15'),
    tags: ['images', 'optimization', 'cdn'],
  },
  {
    id: '39',
    name: 'Queue Worker',
    description: 'Background job processing with Bull, BullMQ, or SQS',
    type: 'template',
    author: 'Block Team',
    downloads: 2670,
    rating: 4.8,
    lastUpdated: new Date('2024-01-25'),
    tags: ['queue', 'workers', 'background-jobs'],
  },
  {
    id: '40',
    name: 'Form Builder',
    description: 'Dynamic form builder with validation and conditional logic',
    type: 'recipe',
    author: 'Community',
    downloads: 1920,
    rating: 4.5,
    lastUpdated: new Date('2024-01-18'),
    tags: ['forms', 'validation', 'ui'],
  },
  {
    id: '41',
    name: 'Notification System',
    description: 'Multi-channel notifications: email, SMS, push, in-app',
    type: 'extension',
    author: 'Block Team',
    downloads: 2890,
    rating: 4.9,
    lastUpdated: new Date('2024-01-26'),
    tags: ['notifications', 'messaging', 'alerts'],
  },
  {
    id: '42',
    name: 'PDF Generator',
    description: 'Generate PDFs from HTML with Puppeteer or PDFKit',
    type: 'recipe',
    author: 'Community',
    downloads: 1450,
    rating: 4.6,
    lastUpdated: new Date('2024-01-16'),
    tags: ['pdf', 'documents', 'generation'],
  },
  {
    id: '43',
    name: 'Analytics Dashboard',
    description: 'Custom analytics with Google Analytics, Mixpanel, or Amplitude',
    type: 'template',
    author: 'Block Team',
    downloads: 3120,
    rating: 4.8,
    lastUpdated: new Date('2024-01-27'),
    tags: ['analytics', 'tracking', 'metrics'],
  },
  {
    id: '44',
    name: 'API Versioning',
    description: 'Implement API versioning strategies and deprecation',
    type: 'recipe',
    author: 'Community',
    downloads: 1680,
    rating: 4.7,
    lastUpdated: new Date('2024-01-20'),
    tags: ['api', 'versioning', 'backwards-compatibility'],
  },
  {
    id: '45',
    name: 'State Machine',
    description: 'Implement state machines with XState or custom logic',
    type: 'extension',
    author: 'Block Team',
    downloads: 1340,
    rating: 4.6,
    lastUpdated: new Date('2024-01-19'),
    tags: ['state-machine', 'workflow', 'xstate'],
  },
  {
    id: '46',
    name: 'Blog Platform',
    description: 'Complete blog platform with CMS, comments, and SEO',
    type: 'template',
    author: 'Community',
    downloads: 2450,
    rating: 4.7,
    lastUpdated: new Date('2024-01-22'),
    tags: ['blog', 'cms', 'content'],
  },
  {
    id: '47',
    name: 'Backup Automation',
    description: 'Automated backups for databases and file systems',
    type: 'recipe',
    author: 'Block Team',
    downloads: 1890,
    rating: 4.8,
    lastUpdated: new Date('2024-01-24'),
    tags: ['backup', 'disaster-recovery', 'automation'],
  },
  {
    id: '48',
    name: 'Chat Application',
    description: 'Real-time chat with rooms, DMs, and file sharing',
    type: 'template',
    author: 'Community',
    downloads: 3340,
    rating: 4.9,
    lastUpdated: new Date('2024-01-28'),
    tags: ['chat', 'messaging', 'realtime'],
  },
];

const HeroCard: React.FC<{
  item: MarketplaceItem;
  onInstall: (item: MarketplaceItem) => void;
}> = ({ item, onInstall }) => {
  const [isHovered, setIsHovered] = useState(false);

  const formatDownloads = (count: number) => {
    if (count >= 1000) return `${(count / 1000).toFixed(1)}k`;
    return count.toString();
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      whileHover={{ scale: 1.002 }}
      onHoverStart={() => setIsHovered(true)}
      onHoverEnd={() => setIsHovered(false)}
      className={`
        relative cursor-pointer overflow-hidden
        col-span-full
        rounded-2xl
        bg-background-default
        min-h-[640px] md:min-h-[500px] sm:min-h-[700px]
      `}
    >

      {/* Tagline badge - Top left */}
      {item.tagline && (
        <div className="absolute top-6 left-6 md:top-12 md:left-12 z-10 inline-block px-3 py-1.5 md:px-4 md:py-2 rounded-full bg-background-accent text-text-on-accent text-xs md:text-sm font-semibold">
          {item.tagline}
        </div>
      )}

      {/* Content - Left side with text - Bottom aligned on desktop, top on mobile */}
      <div className="absolute top-20 left-6 lg:bottom-0 lg:top-auto lg:left-0 z-10 p-6 lg:p-12 max-w-xl">
        <h2 className="text-3xl lg:text-5xl font-light text-text-default mb-4 lg:mb-6">
          {item.name}
        </h2>
        <p className="text-text-muted text-base lg:text-lg mb-6 lg:mb-8 leading-relaxed">
          {item.description}
        </p>

        {/* Stats */}
        <div className="flex flex-wrap items-center gap-4 lg:gap-8 text-text-muted">
          <div className="flex items-center gap-2">
            <Download className="w-4 h-4 lg:w-5 lg:h-5" />
            <span className="font-medium text-sm lg:text-base">{formatDownloads(item.downloads)} downloads</span>
          </div>
          <div className="flex items-center gap-2">
            <Star className="w-4 h-4 lg:w-5 lg:h-5 fill-yellow-500 text-yellow-500" />
            <span className="font-medium text-sm lg:text-base">{item.rating} rating</span>
          </div>
          <div className="px-3 py-1.5 lg:px-4 lg:py-2 rounded-full bg-background-medium text-text-default capitalize text-xs lg:text-sm font-semibold">
            {item.type}
          </div>
        </div>
      </div>

      {/* Hero image - responsive positioning */}
      {/* Different images based on item.image property */}
      {item.image === 'peeps' ? (
        // Custom image positioned on the right bottom (like peeps.png for Enterprise Solutions)
        <div className="absolute right-12 bottom-0 z-0 h-[80%] flex items-end">
          <img 
            src={PeepsImage}
            alt={item.name}
            className="h-full w-auto object-contain object-bottom drop-shadow-2xl"
          />
        </div>
      ) : item.image ? (
        // Other custom images - full width
        <div className="absolute inset-0 z-0">
          <img 
            src={GooseAppImage}
            alt={item.name}
            className="w-full h-full object-cover object-center"
          />
        </div>
      ) : (
        // Default phone image with bottom cutoff (for Goose iOS)
        <div className="absolute bottom-[-20%] left-1/2 -translate-x-1/2 lg:left-auto lg:translate-x-0 lg:right-[10vw] z-10 h-[500px] lg:h-[100%] flex items-start">
          <img 
            src={GooseAppImage} 
            alt="Goose App"
            className="h-full w-auto object-contain object-top drop-shadow-2xl"
          />
        </div>
      )}

      {/* Install button - bottom right */}
      <motion.button
        animate={{ scale: isHovered ? 1.1 : 1 }}
        onClick={(e) => {
          e.stopPropagation();
          onInstall(item);
        }}
        className="absolute bottom-6 right-6 md:bottom-12 md:right-12 z-20 w-12 h-12 md:w-16 md:h-16 rounded-full bg-background-accent text-text-on-accent hover:bg-background-medium hover:text-text-default transition-colors flex items-center justify-center"
      >
        <Download className="w-5 h-5 md:w-7 md:h-7" />
      </motion.button>
    </motion.div>
  );
};

const FeaturedCard: React.FC<{
  item: MarketplaceItem;
  onInstall: (item: MarketplaceItem) => void;
}> = ({ item, onInstall }) => {
  const [isHovered, setIsHovered] = useState(false);

  const formatDownloads = (count: number) => {
    if (count >= 1000) return `${(count / 1000).toFixed(1)}k`;
    return count.toString();
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      whileHover={{ scale: 1.005 }}
      onHoverStart={() => setIsHovered(true)}
      onHoverEnd={() => setIsHovered(false)}
      className={`
        relative cursor-pointer overflow-hidden
        rounded-2xl
        bg-background-default
        flex flex-col
        min-h-[400px]
      `}
    >
      {/* Image Section - Takes up most of the space */}
      <div className={`relative w-full h-[280px] ${item.image ? 'bg-white' : `bg-gradient-to-r ${item.gradient}`} overflow-hidden`}>
        {/* Show actual image if provided, otherwise show gradient with pattern */}
        {item.image === 'apps' ? (
          <img 
            src={AppsImage} 
            alt={item.name}
            className="w-full h-full object-cover object-top"
          />
        ) : item.image === 'g2' ? (
          <img 
            src={G2Image} 
            alt={item.name}
            className="w-full h-full object-cover object-top"
          />
        ) : item.image === 'peeps' ? (
          <img 
            src={PeepsImage} 
            alt={item.name}
            className="w-full h-full object-cover object-top"
          />
        ) : item.image === 'extensions' ? (
          <img 
            src={ExtensionsImage} 
            alt={item.name}
            className="w-full h-full object-cover object-top"
          />
        ) : item.image === 'sellers' ? (
          <img 
            src={SellersImage} 
            alt={item.name}
            className="w-full h-full object-cover object-top"
          />
        ) : (
          /* Animated background pattern for gradient cards */
          <div className="absolute inset-0 opacity-20">
            <div className="absolute inset-0 bg-[radial-gradient(circle_at_50%_50%,rgba(255,255,255,0.8),transparent_50%)]" />
          </div>
        )}
        
        {/* Tagline badge - top left */}
        {item.tagline && (
          <div className="absolute top-4 left-4 px-3 py-1 rounded-full bg-black/80 backdrop-blur-sm text-white text-xs font-semibold">
            {item.tagline}
          </div>
        )}

        {/* Install button - top right */}
        <motion.button
          animate={{ scale: isHovered ? 1.1 : 1 }}
          onClick={(e) => {
            e.stopPropagation();
            onInstall(item);
          }}
          className="absolute top-4 right-4 z-10 w-10 h-10 rounded-full text-white hover:bg-white hover:text-gray-900 transition-colors flex items-center justify-center"
        >
          <Download className="w-5 h-5" />
        </motion.button>
      </div>

      {/* Text Content Section - Compact at bottom */}
      <div className="p-6 flex flex-col flex-1">
        <div className="mb-2">
          <p className="text-xs text-text-muted uppercase tracking-wide font-medium mb-1">
            {item.type}
          </p>
          <h3 className="text-xl font-normal text-text-default mb-2">
            {item.name}
          </h3>
        </div>
        
        <p className="text-sm text-text-muted line-clamp-2 mb-4">
          {item.description}
        </p>

        {/* Stats at bottom */}
        <div className="flex items-center gap-4 text-xs text-text-muted mt-auto">
          <div className="flex items-center gap-1">
            <Star className="w-3 h-3 text-yellow-500 fill-yellow-500" />
            <span>{item.rating}</span>
          </div>
          <div className="flex items-center gap-1">
            <Download className="w-3 h-3" />
            <span>{formatDownloads(item.downloads)}</span>
          </div>
        </div>
      </div>
    </motion.div>
  );
};

const MarketplaceCard: React.FC<{ 
  item: MarketplaceItem;
  onInstall: (item: MarketplaceItem) => void;
}> = ({ item, onInstall }) => {
  const [isHovered, setIsHovered] = useState(false);

  const formatDownloads = (count: number) => {
    if (count >= 1000) return `${(count / 1000).toFixed(1)}k`;
    return count.toString();
  };

  const formatDate = (date: Date) => {
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));
    
    if (days === 0) return 'Today';
    if (days === 1) return 'Yesterday';
    if (days < 7) return `${days}d ago`;
    if (days < 30) return `${Math.floor(days / 7)}w ago`;
    return `${Math.floor(days / 30)}mo ago`;
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 20, scale: 0.9 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      whileHover={{ scale: 1.02 }}
      whileTap={{ scale: 0.98 }}
      onHoverStart={() => setIsHovered(true)}
      onHoverEnd={() => setIsHovered(false)}
      className="
        relative cursor-pointer group
        bg-background-default
        px-6 py-6
        transition-colors duration-200
        hover:bg-background-medium
        aspect-square
        flex flex-col justify-between
        rounded-2xl
      "
    >
      {/* Icon/Type indicator in top left */}
      <div className="relative w-fit">
        <div className="w-12 h-12 bg-background-accent rounded-full flex items-center justify-center">
          <Package className="w-6 h-6 text-text-on-accent" />
        </div>
        
        {/* Type badge */}
        <div className="absolute -bottom-1 -right-1 px-2 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-700 capitalize">
          {item.type}
        </div>
      </div>

      {/* Rating in top right */}
      <div className="absolute top-4 right-4 flex items-center gap-1">
        <Star className="w-4 h-4 text-yellow-500 fill-yellow-500" />
        <span className="text-sm font-medium text-text-default">{item.rating}</span>
      </div>

      {/* Install button (visible on hover) */}
      <motion.button
        initial={{ opacity: 0, y: -10 }}
        animate={{ opacity: isHovered ? 1 : 0, y: isHovered ? 0 : -10 }}
        onClick={(e) => {
          e.stopPropagation();
          onInstall(item);
        }}
        className="absolute top-14 right-4 px-3 py-1.5 rounded-lg bg-green-500 text-white text-sm font-medium hover:bg-green-600 transition-colors flex items-center gap-1"
      >
        <Download className="w-4 h-4" />
        Install
      </motion.button>

      {/* Content at bottom */}
      <div className="mt-auto w-full">
        <h3 className="text-lg font-light text-text-default truncate mb-1">
          {item.name}
        </h3>
        <p className="text-xs text-text-muted line-clamp-2 mb-2">
          {item.description}
        </p>
        
        {/* Stats row */}
        <div className="flex items-center gap-3 text-xs text-text-muted">
          <div className="flex items-center gap-1">
            <Download className="w-3 h-3" />
            {formatDownloads(item.downloads)}
          </div>
          <div className="flex items-center gap-1">
            <Clock className="w-3 h-3" />
            {formatDate(item.lastUpdated)}
          </div>
        </div>
        
        {/* Author */}
        <p className="text-xs text-text-muted mt-1">
          by {item.author}
        </p>
      </div>
    </motion.div>
  );
};

const EmptyMarketplaceTile: React.FC = () => {
  return (
    <div
      className="
        relative
        bg-background-default
        px-6 py-6
        aspect-square
        flex flex-col items-center justify-center
        rounded-2xl
        opacity-50
      "
    >
      <div className="w-8 h-8 rounded-full border-2 border-dashed border-text-muted/30 flex items-center justify-center">
        <div className="w-1 h-1 bg-text-muted/30 rounded-full" />
      </div>
    </div>
  );
};

const MarketplaceView: React.FC = () => {
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedFilter, setSelectedFilter] = useState<'all' | 'recipe' | 'extension' | 'template'>('all');
  const [items] = useState<MarketplaceItem[]>(mockItems);

  const filteredItems = items.filter(item => {
    const matchesSearch = item.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                         item.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
                         item.tags.some(tag => tag.toLowerCase().includes(searchQuery.toLowerCase()));
    const matchesFilter = selectedFilter === 'all' || item.type === selectedFilter;
    return matchesSearch && matchesFilter;
  });

  const handleInstall = (item: MarketplaceItem) => {
    console.log('Installing:', item);
    // TODO: Implement installation logic
  };

  // Calculate empty tiles to fill viewport
  const calculateEmptyTiles = (itemCount: number) => {
    const minTiles = 12; // Minimum 2 rows
    return Math.max(0, minTiles - itemCount);
  };

  return (
    <div className="relative flex flex-col h-screen bg-background-muted">
      {/* Header Section */}
      <div className="pt-14 pb-4 px-4 mb-0.5 bg-background-default rounded-2xl">
        <div className="flex items-center justify-between mb-4">
          <div>
            <h1 className="text-2xl font-light text-text-default mb-1">Marketplace</h1>
            <p className="text-sm text-text-muted">Discover recipes, extensions, and templates</p>
          </div>
          <div className="flex items-center gap-2">
            <button className="px-3 py-2 rounded-lg border border-border-default text-text-default hover:bg-background-medium transition-colors flex items-center gap-2 text-sm">
              <TrendingUp className="w-4 h-4" />
              Trending
            </button>
          </div>
        </div>

        {/* Search and Filters */}
        <div className="flex items-center gap-3">
          <div className="flex-1 relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-text-muted" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search marketplace..."
              className="w-full pl-10 pr-4 py-2 rounded-lg border border-border-default bg-background-muted focus:outline-none focus:ring-2 focus:ring-background-accent text-sm"
            />
          </div>
          
          {/* Filter buttons */}
          <div className="flex items-center gap-2">
            {(['all', 'recipe', 'extension', 'template'] as const).map((filter) => (
              <button
                key={filter}
                onClick={() => setSelectedFilter(filter)}
                className={`px-3 py-2 rounded-lg text-sm font-medium transition-colors capitalize ${
                  selectedFilter === filter
                    ? 'bg-background-accent text-text-on-accent'
                    : 'border border-border-default text-text-default hover:bg-background-medium'
                }`}
              >
                {filter}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {/* Render items in order, grouping featured cards together */}
        {(() => {
          const result: JSX.Element[] = [];
          let featuredBuffer: MarketplaceItem[] = [];
          
          filteredItems.forEach((item, index) => {
            if (item.hero) {
              // Hero card - always full width
              // Flush any buffered featured cards first
              if (featuredBuffer.length > 0) {
                result.push(
                  <div key={`featured-group-${index}`} className="grid grid-cols-1 sm:grid-cols-2 gap-0.5 mb-0.5">
                    {featuredBuffer.map((featuredItem) => (
                      <FeaturedCard
                        key={featuredItem.id}
                        item={featuredItem}
                        onInstall={handleInstall}
                      />
                    ))}
                  </div>
                );
                featuredBuffer = [];
              }
              // Render hero card
              result.push(
                <div key={item.id} className="mb-0.5">
                  <HeroCard item={item} onInstall={handleInstall} />
                </div>
              );
            } else if (item.fullWidth) {
              // Buffer featured cards (half-width cards in 2-column grid)
              featuredBuffer.push(item);
            } else {
              // Flush any buffered featured cards first
              if (featuredBuffer.length > 0) {
                result.push(
                  <div key={`featured-group-${index}`} className="grid grid-cols-1 sm:grid-cols-2 gap-0.5 mb-0.5">
                    {featuredBuffer.map((featuredItem) => (
                      <FeaturedCard
                        key={featuredItem.id}
                        item={featuredItem}
                        onInstall={handleInstall}
                      />
                    ))}
                  </div>
                );
                featuredBuffer = [];
              }
            }
          });
          
          // Flush any remaining featured cards
          if (featuredBuffer.length > 0) {
            result.push(
              <div key="featured-group-final" className="grid grid-cols-1 sm:grid-cols-2 gap-0.5 mb-0.5">
                {featuredBuffer.map((featuredItem) => (
                  <FeaturedCard
                    key={featuredItem.id}
                    item={featuredItem}
                    onInstall={handleInstall}
                  />
                ))}
              </div>
            );
          }
          
          return result;
        })()}

        {/* Regular Tiles Section */}
        {filteredItems.filter(item => !item.hero && !item.fullWidth).length > 0 && (
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-0.5">
            {filteredItems
              .filter(item => !item.hero && !item.fullWidth)
              .map((item) => (
                <MarketplaceCard
                  key={item.id}
                  item={item}
                  onInstall={handleInstall}
                />
              ))}
            
            {/* Empty tiles */}
            {Array.from({ length: calculateEmptyTiles(filteredItems.filter(i => !i.hero && !i.fullWidth).length) }).map((_, index) => (
              <EmptyMarketplaceTile key={`empty-${index}`} />
            ))}
          </div>
        )}

        {/* Empty state */}
        {filteredItems.length === 0 && (
          <div className="text-center py-12">
            <Package className="w-12 h-12 text-text-muted mx-auto mb-4" />
            <h3 className="text-lg font-medium text-text-default mb-2">No items found</h3>
            <p className="text-text-muted">
              Try adjusting your search or filters
            </p>
          </div>
        )}
      </div>
    </div>
  );
};

export default MarketplaceView;
