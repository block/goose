import Link from "@docusaurus/Link";
import { IconDownload } from "@site/src/components/icons/download";

const LinuxDesktopInstallButtons = () => {
  return (
    <div>
      <p>To download Goose Desktop for Linux, click the button below:</p>
      <div className="pill-button">
        <Link
          className="button button--primary button--lg"
          to="https://github.com/block/goose/releases/download/v1.0.29/goose-aarch64-unknown-linux-gnu.tar.bz2"
        >
          <IconDownload /> Linux
        </Link>
      </div>
    </div>
  );
};

export default LinuxDesktopInstallButtons;