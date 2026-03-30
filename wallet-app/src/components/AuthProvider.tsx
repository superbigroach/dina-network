'use client';

import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { onAuthChange, type User } from '@/lib/firebase';

interface AuthContextType {
  user: User | null;
  loading: boolean;
}

const AuthContext = createContext<AuthContextType>({ user: null, loading: true });

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const unsubscribe = onAuthChange((u) => {
      setUser(u);
      setLoading(false);
    });
    // Timeout: don't hang forever if Firebase auth is slow/blocked
    const timeout = setTimeout(() => {
      setLoading(false);
    }, 1000);
    return () => { unsubscribe(); clearTimeout(timeout); };
  }, []);

  return (
    <AuthContext.Provider value={{ user, loading }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  return useContext(AuthContext);
}
