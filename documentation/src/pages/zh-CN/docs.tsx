import React from "react";
import DocsRedirectPage from "@site/src/components/DocsRedirectPage";

export default function ZhCnDocsRoot(): JSX.Element {
  return (
    <DocsRedirectPage
      to="/zh-CN/docs/quickstart"
      message="正在跳转到中文文档..."
    />
  );
}
