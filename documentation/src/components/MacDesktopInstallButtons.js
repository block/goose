import Link from "@docusaurus/Link";
import { IconDownload } from "@site/src/components/icons/download";

const MacDesktopInstallButtons = ({
  introText = "Click one of the buttons below to download goose Desktop for macOS:",
  siliconLabel = "macOS Silicon",
  intelLabel = "macOS Intel",
}) => {
  return (
    <div>
      <p>{introText}</p>
      <div className="pill-button" style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap" }}>
        <Link
          className="button button--primary button--lg"
          to="https://github.com/block/goose/releases/download/stable/Goose.zip"
        >
          <IconDownload /> {siliconLabel}
        </Link>
        <Link
          className="button button--primary button--lg"
          to="https://github.com/block/goose/releases/download/stable/Goose_intel_mac.zip"
        >
          <IconDownload /> {intelLabel}
        </Link>
      </div>
    </div>
  );
};

export default MacDesktopInstallButtons;
