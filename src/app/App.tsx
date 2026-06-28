import { useEffect } from "react";
import { useAuthStore } from "@/stores/auth-store";
import { LoginPage } from "@/features/auth/login-page";
import { AppShell } from "./app-shell";
import { Loader2 } from "lucide-react";

export default function App() {
  const { session, loading, init } = useAuthStore();

  useEffect(() => {
    void init();
  }, [init]);

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin text-primary" />
      </div>
    );
  }

  if (!session) {
    return <LoginPage />;
  }

  return <AppShell />;
}
