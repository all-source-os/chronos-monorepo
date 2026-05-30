"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { Socket, Channel } from "phoenix";

// ─── Singleton socket ────────────────────────────────────────────────────────

let sharedSocket: Socket | null = null;
let socketRefCount = 0;
let currentToken: string | null = null;

/**
 * Resolve the Phoenix socket base URL.
 *
 * Returns `null` when no real backend WS endpoint can be determined. In that
 * case the hook skips connecting entirely instead of hammering a host that has
 * no `/ws` route.
 *
 * Why this matters in production: the dashboard reaches the Query Service over
 * the Next.js `/api/*` proxy using RELATIVE URLs, so `NEXT_PUBLIC_API_URL` is
 * only read server-side and is undefined in the browser. WebSockets can't be
 * tunneled through a Next.js route handler, so without an explicit
 * `NEXT_PUBLIC_WS_URL` there is no backend to reach. Previously this fell back
 * to `window.location.host` — the Vercel frontend (www.all-source.xyz), which
 * has no Phoenix endpoint — producing an endless `wss://.../ws → 404` reconnect
 * storm in the console. Only fall back to the page origin for local dev, where
 * the dev server fronts the Query Service.
 */
function getSocketUrl(): string | null {
  if (process.env.NEXT_PUBLIC_WS_URL) return process.env.NEXT_PUBLIC_WS_URL;
  if (typeof window !== "undefined") {
    const { hostname, protocol, host } = window.location;
    const isLocalDev = hostname === "localhost" || hostname === "127.0.0.1";
    if (!isLocalDev) {
      // Production with no configured WS URL: realtime is not wired. Bail out
      // rather than retry-loop against a host that 404s the upgrade.
      return null;
    }
    const proto = protocol === "https:" ? "wss:" : "ws:";
    return `${proto}//${host}`;
  }
  return "ws://localhost:3902";
}

async function refreshToken(): Promise<string | null> {
  try {
    const res = await fetch("/api/auth/session");
    if (!res.ok) return null;
    const data = await res.json();
    currentToken = data.token ?? null;
    return currentToken;
  } catch {
    return null;
  }
}

function acquireSocket(): Socket | null {
  if (sharedSocket) {
    socketRefCount++;
    return sharedSocket;
  }

  const base = getSocketUrl();
  // No backend WS endpoint available (e.g. production without NEXT_PUBLIC_WS_URL).
  // Skip connecting so we don't spin a reconnect storm against a 404 host.
  if (!base) return null;

  const url = `${base}/ws`;

  // #3 fix: params as a function so Phoenix re-reads currentToken on every
  // reconnect. refreshToken() updates currentToken before connect, and the
  // onError handler refreshes it on auth failures.
  const socket = new Socket(url, {
    params: () => (currentToken ? { token: currentToken } : {}),
    reconnectAfterMs: (tries: number) =>
      [1000, 2000, 5000, 10000][Math.min(tries - 1, 3)] ?? 10000,
  });

  socket.onError(() => {
    // Token may have expired — refresh so the next reconnect attempt uses a
    // fresh one. refreshToken is async but we don't need to await it here;
    // the Phoenix client's built-in reconnect backoff gives it time to complete.
    refreshToken();
  });

  socket.connect();
  sharedSocket = socket;
  socketRefCount = 1;
  return socket;
}

function releaseSocket(socket: Socket) {
  // #8 fix: only decrement if this is actually the current shared socket.
  // Prevents negative ref counts if cleanup fires for a stale socket reference.
  if (socket !== sharedSocket) return;

  socketRefCount--;
  if (socketRefCount <= 0) {
    sharedSocket.disconnect();
    sharedSocket = null;
    socketRefCount = 0;
  }
}

// ─── Hook ────────────────────────────────────────────────────────────────────

type EventHandlers = Record<string, (payload: unknown) => void>;

interface UsePhoenixChannelOptions {
  /** Shorthand: called on "new_event" messages. Mutually exclusive with `on`. */
  onEvent?: (event: unknown) => void;
  /** Generic event handlers keyed by Phoenix event name. */
  on?: EventHandlers;
  enabled?: boolean;
}

interface UsePhoenixChannelReturn {
  isConnected: boolean;
  send: (event: string, payload: Record<string, unknown>) => void;
  connect: () => void;
}

export function usePhoenixChannel(
  topic: string,
  options: UsePhoenixChannelOptions = {}
): UsePhoenixChannelReturn {
  const { onEvent, on, enabled = true } = options;
  const [isConnected, setIsConnected] = useState(false);
  const channelRef = useRef<Channel | null>(null);
  const socketRef = useRef<Socket | null>(null);
  // #4 fix: guard against concurrent connect() calls while token fetch is in-flight
  const connectingRef = useRef(false);

  // #6 fix: build the full handler map, merging the onEvent shorthand with generic `on`.
  // Stored in a ref so channel.on() always calls the latest handler without re-subscribing.
  const handlersRef = useRef<EventHandlers>({});
  handlersRef.current = on
    ? { ...on }
    : onEvent
      ? { new_event: onEvent }
      : {};

  const connect = useCallback(() => {
    // #4 fix: reject if already connected or a connect is in flight
    if (channelRef.current || connectingRef.current) return;
    connectingRef.current = true;

    refreshToken().then((token) => {
      // Component may have unmounted while we awaited the token
      if (!connectingRef.current) return;

      const socket = acquireSocket();
      // No WS backend configured — leave isConnected=false and don't retry.
      if (!socket) {
        connectingRef.current = false;
        setIsConnected(false);
        return;
      }
      socketRef.current = socket;

      const channel = socket.channel(topic, {});
      channelRef.current = channel;
      connectingRef.current = false;

      channel
        .join()
        .receive("ok", () => setIsConnected(true))
        .receive("error", () => setIsConnected(false));

      // #6 fix: subscribe to all events the caller cares about via the
      // handlers ref. This indirection means we register stable listeners
      // once and they always dispatch through the latest callback.
      const eventNames = new Set([
        ...Object.keys(handlersRef.current),
        // Always subscribe to these so the ref can be updated later
        // without needing to re-join the channel.
        "new_event",
        "state_updated",
        "projection_error",
      ]);

      for (const eventName of eventNames) {
        channel.on(eventName, (payload: unknown) => {
          handlersRef.current[eventName]?.(payload);
        });
      }

      channel.onClose(() => setIsConnected(false));
      channel.onError(() => setIsConnected(false));
    });
  }, [topic]);

  useEffect(() => {
    if (!enabled) return;
    connect();

    return () => {
      // #4 fix: cancel in-flight connect
      connectingRef.current = false;

      if (channelRef.current) {
        channelRef.current.leave();
        channelRef.current = null;
      }
      // #8 fix: pass the specific socket instance so releaseSocket can
      // verify it's still the current shared socket before decrementing.
      if (socketRef.current) {
        releaseSocket(socketRef.current);
        socketRef.current = null;
      }
      setIsConnected(false);
    };
  }, [connect, enabled]);

  const send = useCallback((event: string, payload: Record<string, unknown>) => {
    channelRef.current?.push(event, payload);
  }, []);

  // #5 fix: removed `channel` from return — it was always stale (null) on
  // first render since channel creation is async. Consumers that need the
  // channel instance can use `send()` instead.
  return { isConnected, send, connect };
}
