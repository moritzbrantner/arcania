import { LogIn, UserPlus } from "lucide-react";
import { useState } from "react";
import type { FormEvent } from "react";
import { loginAccount, registerAccount } from "../api";
import { ShellMessage } from "../components/common";
import { authRouteLink } from "../routes";
import type { AuthSessionResponse } from "../types";

export function AuthPage({
  mode,
  nextPath,
  onNavigate,
  onAuthenticated,
}: {
  mode: "register" | "login";
  nextPath: string;
  onNavigate: (to: string) => void;
  onAuthenticated: (session: AuthSessionResponse, nextPath: string) => void;
}) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setNotice(null);
    try {
      const session =
        mode === "register"
          ? await registerAccount(email, password)
          : await loginAccount(email, password);
      onAuthenticated(session, nextPath);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Could not authenticate");
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="app-shell picker-shell">
      <section className="match-picker auth-panel" aria-label="Account access">
        <div>
          <p className="eyebrow">Arcania</p>
          <h1>{mode === "register" ? "Create Account" : "Sign In"}</h1>
        </div>
        <form className="auth-form" onSubmit={handleSubmit}>
          <label htmlFor="auth-email">Email</label>
          <input
            id="auth-email"
            type="email"
            value={email}
            onChange={(event) => setEmail(event.target.value)}
            autoComplete="email"
            required
          />
          <label htmlFor="auth-password">Password</label>
          <input
            id="auth-password"
            type="password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            autoComplete={mode === "register" ? "new-password" : "current-password"}
            minLength={
              mode === "register" && !email.trim().toLocaleLowerCase().endsWith("@local.dev")
                ? 8
                : 1
            }
            required
          />
          <button className="primary-button" type="submit" disabled={busy}>
            {mode === "register" ? <UserPlus size={18} /> : <LogIn size={18} />}
            {mode === "register" ? "Create Account" : "Sign In"}
          </button>
        </form>
        <a
          className="secondary-link"
          href={authRouteLink(mode === "register" ? "login" : "register", nextPath)}
          onClick={(event) => {
            event.preventDefault();
            if (busy) {
              return;
            }
            setNotice(null);
            onNavigate(authRouteLink(mode === "register" ? "login" : "register", nextPath));
          }}
          aria-disabled={busy}
        >
          {mode === "register" ? "I already have an account" : "Create a new account"}
        </a>
        {notice ? <p className="notice">{notice}</p> : null}
      </section>
    </main>
  );
}
