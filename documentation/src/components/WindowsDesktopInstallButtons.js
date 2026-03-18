import Link from "@docusaurus/Link";
import { IconDownload } from "@site/src/components/icons/download";

const WindowsDesktopInstallButtons = ({
  introText = "Click one of the buttons below to download goose Desktop for Windows:",
  windowsLabel = "Windows",
}) => {
  return (
    <div>
      <p>{introText}</p>
      <div className="pill-button">
        <Link
          className="button button--primary button--lg"
          to="https://github.com/block/goose/releases/download/stable/Goose-win32-x64.zip"
        >
          <IconDownload /> {windowsLabel}
        </Link>
      </div>
    </div>
  );
};

export default WindowsDesktopInstallButtons;
