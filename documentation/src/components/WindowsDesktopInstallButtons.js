import Link from "@docusaurus/Link";
import { IconDownload } from "@site/src/components/icons/download";

const WindowsDesktopInstallButtons = () => {
  return (
    <div>
      <p>To download Goose Desktop for Windows, click the buttons below:</p>
      <div className="pill-button">
        <Link
          className="button button--primary button--lg"
          to="https://github.com/block/goose/releases/download/stable/Goose.zip"
        >
          <IconDownload /> Windows
        </Link>
      </div>
    </div>
  );
};

export default WindowsDesktopInstallButtons;
