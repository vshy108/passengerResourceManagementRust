import {
  createContext,
  useContext,
  useEffect,
  useState,
  type JSX,
  type ReactNode,
} from "react";
import { api } from "../services/api";

interface AuthCtx {
  token: string | null;
  login: (token: string) => void;
  logout: () => void;
}

const AuthContext = createContext<AuthCtx>({
  token: null,
  login: () => {},
  logout: () => {},
});

export function AuthProvider({ children }: { children: ReactNode }): JSX.Element {
  // Prefer sessionStorage (survives page refresh) then VITE_API_TOKEN (e2e / dev).
  const [token, setToken] = useState<string | null>(
    () => sessionStorage.getItem("prms_token") ?? api.getToken(),
  );

  useEffect(() => {
    if (token) {
      api.setToken(token);
      sessionStorage.setItem("prms_token", token);
    } else {
      api.setToken(null);
      sessionStorage.removeItem("prms_token");
    }
  }, [token]);

  const login = (t: string): void => setToken(t);

  const logout = (): void => {
    setToken(null);
    window.location.hash = "#/login";
  };

  return (
    <AuthContext.Provider value={{ token, login, logout }}>
      {children}
    </AuthContext.Provider>
  );
}

// eslint-disable-next-line react-refresh/only-export-components
export const useAuth = (): AuthCtx => useContext(AuthContext);
