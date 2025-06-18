import Link from "@docusaurus/Link";
import { IconDownload } from "@site/src/components/icons/download";

const DesktopInstallButtons = () => {
  return (
    <div>
      <p>To download Goose Desktop for macOS, click one of the buttons below:</p>
      <div className="pill-button">
        <Link
          className="button button--primary button--lg"
          to="https://github.com/block/goose/releases/download/stable/Goose.zip"
        >
          <IconDownload /> macOS Silicon
        </Link>
        <Link
          className="button button--primary button--lg"
          to="https://github.com/block/goose/releases/download/stable/Goose_intel_mac.zip"
        >
          <IconDownload /> macOS Intel
        </Link>
      </div>
    </div>
  );
};

export default DesktopInstallButtons;
