// Finding Snag on 127.0.0.1 and connecting to it. Shared by background.js and the tests (which
// pass a fake `fetch`).

const PORTS = [47321, 47322, 47323, 47324, 47325, 47326];
const NOT_CONNECTED = "Not connected to Snag: click Connect in the extension's popup";

/**
 * @param {typeof fetch} fetchFn
 * @param {{ token(): Promise<string>, saveToken(token: string): Promise<void>, saveAccent(accent: string): void }} store
 */
function makeConnection(fetchFn, store) {
  /** First port that answers as Snag: { port, paired } or null. */
  async function findApp(token) {
    for (const port of PORTS) {
      try {
        const resp = await fetchFn(`http://127.0.0.1:${port}/ping`, {
          headers: token ? { "X-RDM-Token": token } : {},
          signal: AbortSignal.timeout(800),
        });
        if (!resp.ok) continue;
        const body = await resp.json();
        if (body.app === "rdm") {
          // Remember Snag's colour for the in-page button and the popup.
          if (body.accent) store.saveAccent(body.accent);
          return { port, paired: Boolean(body.paired) };
        }
      } catch (_) {
        // Nothing on this port: try the next one.
      }
    }
    return null;
  }

  /** One-click pairing: Snag asks the user "Allow?", then hands over the code. */
  async function pair() {
    const app = await findApp("");
    if (!app) throw new Error("Snag isn't running");
    const resp = await fetchFn(`http://127.0.0.1:${app.port}/pair`, { method: "POST" });
    const body = await resp.json().catch(() => ({}));
    if (!resp.ok || !body.token) throw new Error(body.error || "Not allowed in Snag");
    await store.saveToken(body.token);
    return body.token;
  }

  /**
   * Snag and the pairing code. Only something the user just clicked may connect first
   * (`mayPair`: Snag then asks "Allow?"); background work never puts that question up.
   */
  async function pairedApp({ mayPair = false } = {}) {
    const token = await store.token();
    const app = await findApp(token);
    if (!app) throw new Error("Snag isn't running");
    if (app.paired) return { app, token };
    if (!mayPair) throw new Error(NOT_CONNECTED);
    const fresh = await pair();
    const now = await findApp(fresh);
    if (!now || !now.paired) throw new Error("Couldn't connect to Snag");
    return { app: now, token: fresh };
  }

  return { findApp, pair, pairedApp };
}

if (typeof module !== "undefined") module.exports = { makeConnection, NOT_CONNECTED, PORTS };
