export type SharedMatchRoute = {
  matchId: string;
  seatToken: string;
};

function deploymentBasePath(baseUrl: string) {
  const trimmed = baseUrl.replace(/^\/+|\/+$/g, "");
  return trimmed ? `/${trimmed}` : "";
}

export function routePathFromBrowserLocation(
  pathname: string,
  search = "",
  baseUrl = import.meta.env.BASE_URL,
) {
  const basePath = deploymentBasePath(baseUrl);
  const routePath =
    basePath && (pathname === basePath || pathname.startsWith(`${basePath}/`))
      ? pathname.slice(basePath.length) || "/"
      : pathname;
  return `${routePath}${search}`;
}

export function browserRoutePath(path: string, baseUrl = import.meta.env.BASE_URL) {
  const basePath = deploymentBasePath(baseUrl);
  if (!basePath || !path.startsWith("/") || path.startsWith("//")) {
    return path;
  }

  const [pathname] = path.split(/[?#]/, 1);
  if (pathname === basePath || pathname.startsWith(`${basePath}/`)) {
    return path;
  }

  return path === "/" ? `${basePath}/` : `${basePath}${path}`;
}

export function installDeploymentBaseHistory(baseUrl = import.meta.env.BASE_URL) {
  const basePath = deploymentBasePath(baseUrl);
  if (!basePath) {
    return;
  }

  const originalPushState = window.history.pushState.bind(window.history);
  const originalReplaceState = window.history.replaceState.bind(window.history);
  const rewrite = (url: string | URL | null | undefined) =>
    typeof url === "string" ? browserRoutePath(url, baseUrl) : url;
  const pushState: History["pushState"] = (data, unused, url) =>
    originalPushState(data, unused, rewrite(url));
  const replaceState: History["replaceState"] = (data, unused, url) =>
    originalReplaceState(data, unused, rewrite(url));

  window.history.pushState = pushState;
  window.history.replaceState = replaceState;
}

export function currentRoutePath() {
  return routePathFromBrowserLocation(window.location.pathname, window.location.search);
}

export function routeFromPath(path: string) {
  const url = new URL(path, window.location.origin);
  return {
    pathname: url.pathname,
    searchParams: url.searchParams,
  };
}

export function safeAuthNextPath(value: string | null) {
  if (!value || !value.startsWith("/") || value.startsWith("//")) {
    return "/";
  }

  try {
    const nextUrl = new URL(value, window.location.origin);
    if (nextUrl.origin !== window.location.origin) {
      return "/";
    }

    const normalizedNextPath = nextUrl.pathname.replace(/\/+$/, "");
    if (normalizedNextPath === "/login" || normalizedNextPath === "/register") {
      return "/";
    }

    return `${nextUrl.pathname}${nextUrl.search}${nextUrl.hash}`;
  } catch {
    return "/";
  }
}

export function authRouteLink(mode: "register" | "login", nextPath: string) {
  const path = mode === "register" ? "/register" : "/login";
  return nextPath === "/" ? path : `${path}?next=${encodeURIComponent(nextPath)}`;
}

export function protectedLoginRoute(nextPath: string) {
  return `/login?next=${encodeURIComponent(nextPath)}`;
}

export function matchRouteFromPath(path: string) {
  const normalized = path.replace(/\/+$/, "");
  const match = normalized.match(/^\/match\/([^/]+)$/);
  return match ? decodeURIComponent(match[1]) : null;
}

export function replayRouteFromPath(path: string) {
  const normalized = path.replace(/\/+$/, "");
  const match = normalized.match(/^\/matches\/([^/]+)\/replay$/);
  return match ? decodeURIComponent(match[1]) : null;
}

export function matchSummaryRouteFromPath(path: string) {
  const normalized = path.replace(/\/+$/, "");
  const match = normalized.match(/^\/matches\/([^/]+)\/summary$/);
  return match ? decodeURIComponent(match[1]) : null;
}

export function sharedMatchSummaryRouteFromPath(path: string) {
  const normalized = path.replace(/\/+$/, "");
  const match = normalized.match(/^\/match\/([^/]+)\/([^/]+)\/summary$/);
  return match
    ? {
        matchId: decodeURIComponent(match[1]),
        seatToken: decodeURIComponent(match[2]),
      }
    : null;
}

export function sharedReplayRouteFromPath(path: string) {
  const normalized = path.replace(/\/+$/, "");
  const match = normalized.match(/^\/match\/([^/]+)\/([^/]+)\/replay$/);
  return match
    ? {
        matchId: decodeURIComponent(match[1]),
        seatToken: decodeURIComponent(match[2]),
      }
    : null;
}

export function sharedMatchRouteFromPath(path: string) {
  const normalized = path.replace(/\/+$/, "");
  const match = normalized.match(/^\/match\/([^/]+)\/([^/]+)$/);
  return match
    ? {
        matchId: decodeURIComponent(match[1]),
        seatToken: decodeURIComponent(match[2]),
      }
    : null;
}

export function publicDeckRouteFromPath(path: string) {
  const normalized = path.replace(/\/+$/, "");
  const match = normalized.match(/^\/@([^/]+)\/decks\/(\d+)$/);
  if (!match) {
    return null;
  }

  return {
    handle: decodeURIComponent(match[1]),
    deckId: Number(match[2]),
  };
}

export function wikiTopicSlugFromPath(path: string) {
  const [pathname] = path.split(/[?#]/);
  const normalized = pathname.replace(/\/+$/, "");
  if (normalized === "/wiki") {
    return null;
  }

  const match = normalized.match(/^\/wiki\/([^/]+)$/);
  return match ? decodeURIComponent(match[1]) : null;
}
