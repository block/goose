import React, { useState, useEffect } from 'react';
import { Copy, Check, Download, Terminal } from 'lucide-react';
import styles from './styles.module.css';

interface GitHubStats {
  stars: string;
  contributors: string;
  loading: boolean;
}

export default function HeroInstall() {
  const [expandedSection, setExpandedSection] = useState<'desktop' | 'cli' | null>(null);
  const [platform, setPlatform] = useState('macOS');
  const [copied, setCopied] = useState(false);
  const [stats, setStats] = useState<GitHubStats>({
    stars: '12.5K',
    contributors: '450+',
    loading: true,
  });

  useEffect(() => {
    // Auto-detect platform
    const userAgent = navigator.platform.toLowerCase();
    if (userAgent.includes('mac')) setPlatform('macOS');
    else if (userAgent.includes('win')) setPlatform('Windows');
    else if (userAgent.includes('linux')) setPlatform('Linux');

    // Fetch GitHub stats
    const fetchStats = async () => {
      try {
        const [repoResponse, contributorsResponse] = await Promise.all([
          fetch('https://api.github.com/repos/block/goose'),
          fetch('https://api.github.com/repos/block/goose/contributors?per_page=1&anon=true')
        ]);

        const repoData = await repoResponse.json();
        const contributorsHeader = contributorsResponse.headers.get('Link');

        // Parse contributor count from Link header
        let contributorCount = 450; // fallback
        if (contributorsHeader) {
          const match = contributorsHeader.match(/page=(\d+)>; rel="last"/);
          if (match) contributorCount = parseInt(match[1]);
        }

        setStats({
          stars: formatNumber(repoData.stargazers_count),
          contributors: `${formatNumber(contributorCount)}+`,
          loading: false,
        });
      } catch (error) {
        // Keep default values on error
        setStats(prev => ({ ...prev, loading: false }));
      }
    };

    fetchStats();
  }, []);

  const handleCopy = () => {
    navigator.clipboard.writeText('curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash');
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const toggleSection = (section: 'desktop' | 'cli') => {
    setExpandedSection(expandedSection === section ? null : section);
  };

  const getDownloadLabel = () => {
    if (platform === 'macOS') return 'Download for macOS';
    if (platform === 'Windows') return 'Download for Windows';
    if (platform === 'Linux') return 'Download for Linux';
    return 'Download Desktop App';
  };

  return (
    <div className={styles.heroInstall}>
      <div className={styles.buttonGroup}>
        <button
          className={`${styles.primaryButton} ${expandedSection === 'desktop' ? styles.buttonActive : ''}`}
          onClick={() => toggleSection('desktop')}
        >
          <Download size={20} />
          {getDownloadLabel()}
        </button>
        <button
          className={`${styles.secondaryButton} ${expandedSection === 'cli' ? styles.buttonActive : ''}`}
          onClick={() => toggleSection('cli')}
        >
          <Terminal size={20} />
          Install CLI
        </button>
      </div>

      {/* Desktop Downloads */}
      {expandedSection === 'desktop' && (
        <div className={styles.expandedContent}>
          {platform === 'macOS' && (
            <>
              <div className={styles.downloadOptions}>
                <a
                  href="https://github.com/block/goose/releases/latest/download/goose_darwin_arm64.zip"
                  className={styles.downloadLink}
                >
                  <Download size={24} className={styles.downloadIcon} />
                  <div className={styles.downloadInfo}>
                    <div className={styles.downloadName}>Apple Silicon</div>
                    <div className={styles.downloadDesc}>M1, M2, M3, M4 chips</div>
                  </div>
                </a>
                <a
                  href="https://github.com/block/goose/releases/latest/download/goose_darwin_amd64.zip"
                  className={styles.downloadLink}
                >
                  <Download size={24} className={styles.downloadIcon} />
                  <div className={styles.downloadInfo}>
                    <div className={styles.downloadName}>Intel</div>
                    <div className={styles.downloadDesc}>Intel-based Macs</div>
                  </div>
                </a>
              </div>
            </>
          )}

          {platform === 'Windows' && (
            <>
              <div className={styles.downloadOptions}>
                <a
                  href="https://github.com/block/goose/releases/latest/download/goose_windows_amd64.zip"
                  className={styles.downloadLink}
                >
                  <Download size={24} className={styles.downloadIcon} />
                  <div className={styles.downloadInfo}>
                    <div className={styles.downloadName}>Windows (64-bit)</div>
                    <div className={styles.downloadDesc}>Windows 10/11</div>
                  </div>
                </a>
              </div>
            </>
          )}

          {platform === 'Linux' && (
            <>
              <div className={styles.downloadOptions}>
                <a
                  href="https://github.com/block/goose/releases/latest/download/goose_linux_amd64.deb"
                  className={styles.downloadLink}
                >
                  <Download size={24} className={styles.downloadIcon} />
                  <div className={styles.downloadInfo}>
                    <div className={styles.downloadName}>DEB Package</div>
                    <div className={styles.downloadDesc}>Ubuntu, Debian, Mint</div>
                  </div>
                </a>
                <a
                  href="https://github.com/block/goose/releases/latest/download/goose_linux_amd64.rpm"
                  className={styles.downloadLink}
                >
                  <Download size={24} className={styles.downloadIcon} />
                  <div className={styles.downloadInfo}>
                    <div className={styles.downloadName}>RPM Package</div>
                    <div className={styles.downloadDesc}>Fedora, RHEL, CentOS</div>
                  </div>
                </a>
              </div>
            </>
          )}

          <a href="/docs/getting-started/installation" className={styles.installLink}>
            View full installation guide →
          </a>
        </div>
      )}

      {/* CLI Installation */}
      {expandedSection === 'cli' && (
        <div className={styles.expandedContent}>
          <div className={styles.codeBlock}>
            <code>curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash</code>
            <button className={styles.copyButton} onClick={handleCopy}>
              {copied ? <Check size={16} /> : <Copy size={16} />}
            </button>
          </div>
          <a href="/docs/getting-started/installation" className={styles.installLink}>
            View full installation guide →
          </a>
        </div>
      )}

      <p className={styles.stats}>
        ⭐ {stats.stars} stars • 👥 {stats.contributors} contributors
      </p>
    </div>
  );
}

function formatNumber(num: number): string {
  if (num >= 1000000) return (num / 1000000).toFixed(1) + 'M';
  if (num >= 1000) return (num / 1000).toFixed(1) + 'K';
  return num.toString();
}
