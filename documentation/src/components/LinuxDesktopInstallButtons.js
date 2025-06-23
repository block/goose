import Link from "@docusaurus/Link";
import { IconDownload } from "@site/src/components/icons/download";

const LinuxDesktopInstallButtons = () => {
  return (
    <div>
      <p>To download Goose Desktop for Linux, choose the buttons below:</p>
      <div className="pill-button" style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap' }}>
        <Link
          className="button button--primary button--lg"
          to="https://github.com/block/goose/releases/download/v1.0.29/goose-x86_64-unknown-linux-gnu.tar.bz2"
        >
          <IconDownload /> Linux x86_64
        </Link>
        <Link
          className="button button--primary button--lg"
          to="https://github.com/block/goose/releases/download/v1.0.29/goose-aarch64-unknown-linux-gnu.tar.bz2"
        >
          <IconDownload /> Linux ARM64
        </Link>
        <Link
          className="button button--primary button--lg"
          to="https://github.com/block/goose/releases/download/v1.0.29/Goose-1.0.29-1.x86_64.rpm"
        >
          <IconDownload /> RPM Package
        </Link>
      </div>
    </div>
  );
};

export default LinuxDesktopInstallButtons;