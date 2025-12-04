import React from "react";
import Admonition from '@theme/Admonition';

export const PlatformExtensionNote = () => {
  
  return (
    <Admonition type="info" title="Platform Extension">
       <p>This is a <a href="/goose/docs/getting-started/using-extensions#built-in-platform-extensions">built-in platform extension</a> that's enabled by default. Platform extensions provide core functionality and are used within goose just like MCP server extensions.</p>
    </Admonition>
  );
};
