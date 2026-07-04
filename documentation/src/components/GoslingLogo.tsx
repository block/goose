import { useColorMode } from '@docusaurus/theme-common';

export const GoslingLogo = (props: { className?: string }) => {
  const { colorMode } = useColorMode();

  const logoSrc = colorMode === 'dark' ? 'img/logo_dark.svg' : 'img/logo_light.svg';

  const logoAlt = 'gosling logo';

  return (
    <img
      src={logoSrc}
      alt={logoAlt}
      className={props.className}
      style={{ height: 'auto', maxWidth: '100%' }}
    />
  );
};
