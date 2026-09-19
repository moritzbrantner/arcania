import { useEffect } from "react";
import { ShellMessage } from "../components/common";

export function RouteRedirect({ to, onNavigate }: { to: string; onNavigate: (to: string) => void }) {
  useEffect(() => {
    onNavigate(to);
  }, [onNavigate, to]);

  return <ShellMessage title="Arcania" message="Opening account" />;
}
