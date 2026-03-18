import React, {useEffect} from "react";
import useBaseUrl from "@docusaurus/useBaseUrl";

type DocsRedirectPageProps = {
  to: string;
  message: string;
};

export default function DocsRedirectPage({
  to,
  message,
}: DocsRedirectPageProps): JSX.Element {
  const target = useBaseUrl(to);

  useEffect(() => {
    window.location.replace(target);
  }, [target]);

  return (
    <main className="container margin-vert--xl">
      <p>{message}</p>
    </main>
  );
}
