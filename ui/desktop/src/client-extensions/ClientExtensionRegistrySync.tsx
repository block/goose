import { useEffect } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { useClientExtensions } from './ClientExtensionsContext';
import { parseClientExtensionViewPath } from './routes';

export function ClientExtensionRegistrySync() {
  const location = useLocation();
  const navigate = useNavigate();
  const { extensions, enabledExtensions } = useClientExtensions();

  useEffect(() => {
    const view = parseClientExtensionViewPath(location.pathname);
    if (!view) {
      return;
    }

    const extension = extensions.find((entry) => entry.id === view.extensionId);
    const enabled = enabledExtensions.some((entry) => entry.id === view.extensionId);

    if (!extension || !enabled) {
      navigate('/pair', { replace: true });
    }
  }, [enabledExtensions, extensions, location.pathname, navigate]);

  return null;
}
