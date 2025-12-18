import React from "react";
import { PanelLeft } from "lucide-react";

export const DesktopAutoUpdateSteps = () => {
  return (
    <>
      <p><strong>To automatically download and install updates:</strong></p>
      <ol>
        <li>Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar</li>
        <li>Click <code>Settings</code> in the sidebar</li>
        <li>Click <code>App</code></li>
        <li>Scroll down to the <code>Updates</code> section:
          <ul>
            <li>Check if your current version is "up to date" or if a newer version is available</li>
            <li>To automatically download the newer version, click <code>Check for Updates</code></li>
          </ul>
        </li>
        <li>Click <code>Install & Restart</code> to immediately relaunch goose Desktop and apply the update<br />
            <strong>Note:</strong> If the app can't be automatically updated, you'll be prompted to manually install the downloaded version</li>
      </ol>
    </>
  );
};
