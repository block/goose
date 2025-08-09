import type { Route } from './+types/home';
import { Welcome } from '../welcome/welcome';

export function meta(_args: Route.MetaArgs) {
  const appName = '__APP_NAME__';
  return [
    { title: `${appName}` },
    { name: 'description', content: `${appName} — built with Goose` },
  ];
}

export default function Home(_props: Route.ComponentProps) {
  return <Welcome />;
}
