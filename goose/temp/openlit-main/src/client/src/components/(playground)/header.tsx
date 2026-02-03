"use client";
import { useEffect, useRef } from "react";
import RouteBreadcrumbs from "./route-breadcrumbs";
import { usePortal } from "./header-portal";

export default function Header() {
	const { setHeaderRef } = usePortal();
  const containerRef = useRef(null);

  useEffect(() => {
    if (containerRef.current) {
      setHeaderRef(containerRef.current);
    }
  }, [setHeaderRef]);

	return (
		<header className="flex w-full mb-2">
			<RouteBreadcrumbs />
			<div ref={containerRef} />
		</header>
	);
}
