import Link from "@docusaurus/Link";
import { IconDownload } from "@site/src/components/icons/download";

const LinuxDesktopInstallButtons = () => {
  return (
    <div>
      <p>To download Goose Desktop for Linux, choose the buttons below:</p>
      <div className="pill-button" style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap' }}>
        <Link
          className="button button--primary button--lg"
          to="https://github.com/block/goose/releases/download/stable/goose_1.0.29_amd64.deb"
        >
          <IconDownload /> DEB Package (Ubuntu/Debian)
        </Link>
        <Link
          className="button button--primary button--lg"
          to="https://github.com/block/goose/releases/download/stable/Goose-1.0.29-1.x86_64.rpm"
        >
          <IconDownload /> RPM Package (RHEL/Fedora)
        </Link>
      </div>
    </div>
  );
};

export default LinuxDesktopInstallButtons;