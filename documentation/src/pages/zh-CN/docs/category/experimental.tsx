import React from "react";
import DocsRedirectPage from "@site/src/components/DocsRedirectPage";

export default function ZhCnExperimentalCategoryAlias(): JSX.Element {
  return (
    <DocsRedirectPage
      to="/zh-CN/docs/experimental"
      message="正在跳转到中文“实验功能”分类..."
    />
  );
}
