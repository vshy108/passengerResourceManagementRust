import { type JSX } from "react";
import { AuthProvider, useAuth } from "./contexts/AuthContext";
import { DataProvider } from "./contexts/DataContext";
import { AppShell } from "./components/AppShell";
import { LoginPage } from "./components/LoginPage";
import { useHash } from "./hooks/useHash";

function AppInner(): JSX.Element {
  const { token } = useAuth();
  const hash = useHash();

  // Show login when there is no token or the user explicitly navigated to /login.
  if (!token || hash === "#/login") {
    return <LoginPage />;
  }

  return (
    <DataProvider>
      <AppShell />
    </DataProvider>
  );
}

export function App(): JSX.Element {
  return (
    <AuthProvider>
      <AppInner />
    </AuthProvider>
  );
}
