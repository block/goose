import React from "react";
import Admonition from "@theme/Admonition";

const SupportedEnvironments = () => {
  return (
    <Admonition type="info" title="Supported Environments">
      Goose currently works on <strong>macOS</strong> and <strong>Linux</strong> systems and supports both <strong>ARM</strong> and <strong>x86</strong> architectures. If you'd like to request support for additional operating systems, please{" "}
      <a
        href="https://github.com/block/goose/issues/new?template=Blank+issue"
        target="_blank"
        rel="noopener noreferrer"
      >
        open an issue on GitHub
      </a>.
    </Admonition>
  );
};

export default SupportedEnvironments;
