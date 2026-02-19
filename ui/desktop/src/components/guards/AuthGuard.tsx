import { Navigate } from 'react-router-dom';
import { useAuth } from '../../hooks/useAuth';

export function AuthGuard({ children }: { children: React.ReactNode }) {
  const { isAuthenticated, isLoading, authRequired } = useAuth();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center w-full h-full bg-background-default">
        <div className="animate-spin rounded-full h-8 w-8 border-2 border-border-strong border-t-transparent" />
      </div>
    );
  }

  if (!authRequired) {
    return <>{children}</>;
  }

  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }

  return <>{children}</>;
}
