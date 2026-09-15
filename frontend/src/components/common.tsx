import { BookOpen, Gauge, Layers, LibraryBig, LogIn, LogOut, History, Menu, Play, Settings as SettingsIcon, User, UsersRound, X } from "lucide-react";
import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import type { AccountProps } from "../appTypes";
import { avatarSymbolLabel } from "../labels";
import { protectedLoginRoute } from "../routes";

export function DetailStat({ label, value }: { label: string; value: string | number }) {
  return (
    <span className="detail-stat">
      <span>{label}</span>
      <strong>{value}</strong>
    </span>
  );
}


export function AccountActions({
  currentUser,
  onSignOut,
  onNavigate,
  allowSignOut = true,
  loginNextPath,
  activeAccountRoute = null,
}: AccountProps) {
  const [menuOpen, setMenuOpen] = useState(false);
  const menuId = useId();
  const containerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!menuOpen) {
      return;
    }

    function handlePointerDown(event: PointerEvent) {
      if (!containerRef.current?.contains(event.target as Node)) {
        setMenuOpen(false);
      }
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setMenuOpen(false);
      }
    }

    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [menuOpen]);

  if (!currentUser) {
    const loginPath = loginNextPath ? protectedLoginRoute(loginNextPath) : "/login";
    return (
      <div className="account-actions">
        <button className="secondary-link" type="button" onClick={() => onNavigate(loginPath)}>
          <LogIn size={18} />
          Sign In
        </button>
      </div>
    );
  }

  return (
    <div className="account-actions account-menu-root" ref={containerRef}>
      <button
        className="secondary-link account-menu-trigger"
        type="button"
        onClick={() => setMenuOpen((open) => !open)}
        aria-haspopup="menu"
        aria-expanded={menuOpen}
        aria-controls={menuId}
        aria-label={`Account menu for ${currentUser.displayName}`}
      >
        <span className={`profile-avatar ${currentUser.avatar.color}`}>
          {avatarSymbolLabel(currentUser.avatar.symbol)}
        </span>
        <span className="account-menu-name">{currentUser.displayName}</span>
        <span className="level-badge">Lv. {currentUser.progressionSummary.level}</span>
      </button>
      {menuOpen ? (
        <div className="account-menu" id={menuId} role="menu" aria-label="Account menu">
          <button
            className="account-menu-item"
            type="button"
            role="menuitem"
            disabled={activeAccountRoute === "profile"}
            onClick={() => {
              setMenuOpen(false);
              onNavigate("/profile");
            }}
          >
            <User size={17} />
            Profile
          </button>
          <button
            className="account-menu-item"
            type="button"
            role="menuitem"
            onClick={() => {
              setMenuOpen(false);
              onNavigate("/progression");
            }}
          >
            <Gauge size={17} />
            Progression
          </button>
          <button
            className="account-menu-item"
            type="button"
            role="menuitem"
            disabled={activeAccountRoute === "settings"}
            onClick={() => {
              setMenuOpen(false);
              onNavigate("/settings");
            }}
          >
            <SettingsIcon size={17} />
            Settings
          </button>
          {allowSignOut ? (
            <button
              className="account-menu-item"
              type="button"
              role="menuitem"
              onClick={() => {
                setMenuOpen(false);
                onSignOut();
              }}
            >
              <LogOut size={17} />
              Sign out
            </button>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

export function TopNav({ currentUser, onNavigate, onSignOut, activePath = "", ...accountActionProps }: AccountProps & { activePath?: string }) {
  const activeAccountRoute =
    accountActionProps.activeAccountRoute ??
    (activePath === "/profile" ? "profile" : activePath === "/settings" ? "settings" : null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [learnOpen, setLearnOpen] = useState(false);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const wasDrawerOpen = useRef(false);
  useEffect(() => {
    if (wasDrawerOpen.current && !drawerOpen) {
      triggerRef.current?.focus();
    }
    wasDrawerOpen.current = drawerOpen;
  }, [drawerOpen]);
  useEffect(() => {
    if (!drawerOpen) return;
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setDrawerOpen(false);
    };
    document.addEventListener("keydown", closeOnEscape);
    return () => document.removeEventListener("keydown", closeOnEscape);
  }, [drawerOpen]);
  const navigate = (to: string) => {
    setDrawerOpen(false);
    setLearnOpen(false);
    onNavigate(to);
  };
  const coreLinks = [
    { to: "/", label: "Dashboard", icon: Gauge, signedIn: false },
    { to: "/play", label: "Play", icon: Play, signedIn: false },
    { to: "/decks", label: "Decks", icon: Layers, signedIn: true },
    { to: "/heroes", label: "Heroes", icon: UsersRound, signedIn: true },
    { to: "/matches", label: "Matches", icon: History, signedIn: true },
  ];
  const learnLinks = [
    { to: "/catalog/", label: "Catalog", icon: LibraryBig },
    { to: "/wiki", label: "Rules", icon: BookOpen },
    { to: "/tutorial", label: "Tutorial", icon: BookOpen },
    { to: "/settings", label: "Settings", icon: SettingsIcon },
  ];
  return (
    <nav className="top-nav" aria-label="Primary navigation">
      <div className="top-nav-links">
        {coreLinks.filter((link) => !link.signedIn || currentUser).map(({ to, label, icon: Icon }) => <button key={to} className={`secondary-link ${activePath === to ? "active" : ""}`} type="button" onClick={() => navigate(to)} aria-current={activePath === to ? "page" : undefined}><Icon size={18}/>{label}</button>)}
        <div className="learn-menu-root">
          <button className="secondary-link" type="button" onClick={() => setLearnOpen((open) => !open)} aria-expanded={learnOpen} aria-haspopup="menu">Learn & settings</button>
          {learnOpen ? <div className="learn-menu" role="menu">{learnLinks.map(({ to, label, icon: Icon }) => <button key={to} type="button" role="menuitem" onClick={() => navigate(to)}><Icon size={17}/>{label}</button>)}</div> : null}
        </div>
      </div>
      <AccountActions
        currentUser={currentUser}
        onNavigate={onNavigate}
        onSignOut={onSignOut}
        {...accountActionProps}
        activeAccountRoute={activeAccountRoute}
      />
      <button ref={triggerRef} className="icon-button nav-drawer-trigger" type="button" aria-label="Open navigation menu" aria-expanded={drawerOpen} onClick={() => setDrawerOpen(true)}><Menu size={20}/></button>
      {drawerOpen ? <div className="nav-drawer-backdrop" onMouseDown={() => setDrawerOpen(false)}><div className="nav-drawer" role="dialog" aria-modal="true" aria-label="Navigation menu" onMouseDown={(event) => event.stopPropagation()}><div className="nav-drawer-heading"><strong>Navigation</strong><button className="icon-button" type="button" aria-label="Close navigation menu" onClick={() => setDrawerOpen(false)}><X size={18}/></button></div>{coreLinks.filter((link) => !link.signedIn || currentUser).map(({ to, label, icon: Icon }) => <button key={to} className={activePath === to ? "active" : ""} type="button" onClick={() => navigate(to)}><Icon size={18}/>{label}</button>)}<hr/>{learnLinks.map(({to,label,icon:Icon}) => <button key={to} type="button" onClick={() => navigate(to)}><Icon size={18}/>{label}</button>)}</div></div> : null}
    </nav>
  );
}


export function ShellMessage({
  title,
  message,
  actions,
}: {
  title: string;
  message: string;
  actions?: ReactNode;
}) {
  return (
    <main className="app-shell centered">
      <div className="shell-message">
        <p className="eyebrow">{title}</p>
        <h1>{message}</h1>
        {actions ? <div className="actions shell-actions">{actions}</div> : null}
      </div>
    </main>
  );
}
