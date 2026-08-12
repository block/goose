import { useColorMode } from '@docusaurus/theme-common';

export const GooseLogo = (props: { className?: string }) => {
  const { colorMode } = useColorMode();
  
  const logoSrc = colorMode === 'dark' 
    ? 'img/goose-logo-white.png' 
    : 'img/goose-logo-black.png';
  
  const logoAlt = 'goose logo';

  return (
    <img
      src={logoSrc}
      alt={logoAlt}
      className={props.className}
      style={{ height: 'auto', maxWidth: '100%' }}
    />
  );
};
