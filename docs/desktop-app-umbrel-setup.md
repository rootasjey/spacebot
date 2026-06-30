# Connecter l'app desktop Spacebot à une instance Umbrel distante

## Contexte

Spacebot est un agent AI open-source (Rust). Il peut tourner sur un serveur Umbrel via le store Umbrel. L'app desktop (Tauri 2 + frontend React) permet d'interagir avec Spacebot depuis macOS.

Le défi : l'app desktop est conçue pour communiquer avec une API HTTP locale (localhost:19898), mais l'instance tourne sur un serveur distant protégé par l'authentification Umbrel (Caddy reverse proxy + session cookie Umbrel).

## Prérequis

- **macOS** (Apple Silicon ou Intel)
- **Homebrew** (`brew`)
- **Rust** 1.85+ via `rustup`
- **Bun** — runtime JS/TS (alternative plus rapide à Node.js)
- **Xcode** (pour les outils de compilation macOS)
- **Tailscale** (optionnel, pour accéder au serveur Umbrel à distance)
- **Instance Spacebot** fonctionnelle sur Umbrel
- **Mot de passe Umbrel** (celui de l'écran de login)

## 1. Installer les dépendances

```bash
# Rust (mise à jour)
rustup update stable

# Bun
brew install oven-sh/bun/bun

# Xcode Command Line Tools (déjà présent si Xcode est installé)
xcode-select --install
```

## 2. Cloner et builder l'app desktop

```bash
git clone https://github.com/spacedriveapp/spacebot
cd spacebot

# Installer les dépendances frontend
cd interface && bun install --frozen-lockfile && cd ..

# Installer le CLI Tauri
cd desktop && bun install --frozen-lockfile && cd ..
```

Pour builder sans le binaire serveur (sidecar) — on utilisera une instance distante :

Modifier `desktop/package.json` :

```diff
-    "tauri:build": "../scripts/bundle-sidecar.sh --release && tauri build"
+    "tauri:build": "tauri build"
```

Modifier `desktop/src-tauri/tauri.conf.json` (supprimer `externalBin` et `resources`) :

```diff
    "bundle": {
      "active": true,
-     "externalBin": ["binaries/spacebot"],
-     "resources": {
-       "gen/icon/**/*": "./"
-     },
      "macOS": {
```

```bash
# Builder l'app
cd desktop && bun run tauri:build
```

Les fichiers générés :
- `desktop/src-tauri/target/release/bundle/macos/Spacebot.app`
- `desktop/src-tauri/target/release/bundle/dmg/Spacebot_0.1.0_aarch64.dmg`

```bash
# Copier dans /Applications
cp -r desktop/src-tauri/target/release/bundle/macos/Spacebot.app /Applications/
```

## 3. Activer l'API Spacebot sur Umbrel

Dans l'interface Umbrel, ouvrir Spacebot → Settings → **Enable API Server** → **ON**.

Vérifier que la config contient bien :

```toml
[api]
bind = "::"
enabled = true
port = 19898
```

Redémarrer Spacebot sur Umbrel après avoir activé l'API.

## 4. Configurer le proxy Umbrel

Umbrel utilise Caddy comme reverse proxy avec authentification devant toutes les apps. Pour que l'app desktop puisse communiquer avec l'API, on utilise un proxy local qui gère l'auth Umbrel.

Créer `umbrel-proxy.ts` à la racine du projet :

```typescript
#!/usr/bin/env bun
const TARGET = "http://100.122.110.56:19898";
const AUTH_BASE = "http://100.122.110.56:2000/v1/account/login";

const PASSWORD = process.env.UMBREL_PASSWORD;
if (!PASSWORD) {
  console.error("Missing UMBREL_PASSWORD environment variable");
  process.exit(1);
}

let cookieJar = "";

async function authenticate(redirectUrl: string): Promise<boolean> {
  const redirect = new URL(redirectUrl);
  const origin = redirect.searchParams.get("origin") ?? "host";
  const app = redirect.searchParams.get("app") ?? "spacebot";
  const path = redirect.searchParams.get("path") ?? "/";
  const authUrl = `${AUTH_BASE}?origin=${encodeURIComponent(origin)}&app=${encodeURIComponent(app)}&path=${encodeURIComponent(path)}`;

  const res = await fetch(authUrl, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ password: PASSWORD }),
  });
  if (res.ok) {
    const setCookie = res.headers.get("set-cookie");
    if (setCookie) {
      cookieJar = setCookie;
      return true;
    }
  }
  return false;
}

Bun.serve({
  port: 19898,
  async fetch(req) {
    const url = new URL(req.url);
    const targetUrl = `${TARGET}${url.pathname}${url.search}`;
    const headers = new Headers(req.headers);
    headers.delete("host");
    if (cookieJar) headers.set("cookie", cookieJar);

    const upstreamRes = await fetch(targetUrl, {
      method: req.method,
      headers,
      body: req.method === "GET" || req.method === "HEAD" ? undefined : req.body,
      redirect: "manual",
    });

    if (upstreamRes.status === 302) {
      const ok = await authenticate(upstreamRes.headers.get("location") ?? "");
      if (!ok) return new Response("Authentication failed", { status: 502 });
      headers.set("cookie", cookieJar);
      const retryRes = await fetch(targetUrl, {
        method: req.method,
        headers,
        body: req.method === "GET" || req.method === "HEAD" ? undefined : req.body,
        redirect: "manual",
      });
      return new Response(retryRes.body, { status: retryRes.status, headers: retryRes.headers });
    }

    return new Response(upstreamRes.body, { status: upstreamRes.status, headers: upstreamRes.headers });
  },
});
```

### Comment fonctionne le proxy

1. Reçoit une requête sur `localhost:19898`
2. La forwarde vers `100.122.110.56:19898` (l'instance Umbrel via Tailscale)
3. Si Caddy répond par une 302 (redirection vers l'auth Umbrel), le proxy :
   - Extrait `origin`, `app` et `path` de l'URL de redirection
   - POST sur `/v1/account/login?origin=...&app=...&path=...` avec le mot de passe Umbrel
   - Stocke le cookie de session reçu
   - Retente la requête originale avec le cookie
4. Toutes les requêtes suivantes incluent le cookie

### Pourquoi le champ `app` est nécessaire en query parameter

Le endpoint tRPC d'Umbrel (`/v1/account/login`) rejette les requêtes sans les paramètres `origin`, `app` et `path` avec l'erreur `"'app' is missing"`. Ces paramètres sont passés par l'URL de redirection de Caddy et doivent être propagés à l'appel d'auth.

## 5. Lancer le proxy et l'app

```bash
# Définir le mot de passe Umbrel
read -s UMBREL_PASSWORD
export UMBREL_PASSWORD

# Lancer le proxy
bun run /Users/rootasjey/GitHub/spacebot/umbrel-proxy.ts

# (dans un autre terminal) Vérifier l'accès API
curl -s http://localhost:19898/api/status
# → {"status":"running","version":"0.5.0",...}
```

Configurer l'app :

```bash
# Créer le fichier de connexion
mkdir -p ~/Library/Application\ Support/sh.spacebot.desktop
cat > ~/Library/Application\ Support/sh.spacebot.desktop/connection.json << EOF
{
  "server_url": "http://localhost:19898"
}
EOF
```

Ouvrir `/Applications/Spacebot.app` (clic-droit > Ouvrir la première fois pour contourner Gatekeeper).

## 6. Notes

- **Gatekeeper** : l'app n'est pas signée Apple Developer, donc la première ouverture nécessite clic-droit > Ouvrir
- **Proxy** : doit tourner tant que l'app est utilisée. Pour le lancer en arrière-plan :
  ```bash
  nohup bun run /Users/rootasjey/GitHub/spacebot/umbrel-proxy.ts > /tmp/proxy.log 2>&1 &
  ```
- **Tailscale** : permet d'accéder à l'instance Umbrel depuis n'importe où. L'adresse Tailscale (`100.x.x.x`) remplace `100.122.110.56` si elle change
- **Sécurité** : l'auth Umbrel reste en place. Le proxy ne contourne pas la sécurité, il s'authentifie automatiquement avec le mot de passe que tu lui fournis
- **Limitation** : nécessite un processus séparé (le proxy). Une meilleure solution (option 2) serait d'intégrer le proxy directement dans l'app Rust/Tauri

## Architectures possibles

```
Option 1 (actuelle) :
┌──────────────────┐     ┌────────────┐     ┌────────────────┐
│ Spacebot Desktop │ ──▶ │ Proxy (Bun) │ ──▶ │ Umbrel (Caddy) │
│ (localhost:19898)│     │(localhost)  │     │ :19898 → Auth  │
└──────────────────┘     └────────────┘     │ → Spacebot API │
                                            └────────────────┘

Option 2 (idéale) :
┌────────────────────────────────┐     ┌────────────────┐
│ Spacebot Desktop (Tauri+Rust)  │ ──▶ │ Umbrel (Caddy) │
│ Proxy intégré (reqwest + jar)  │     │ :19898 → Auth  │
│ API calls via IPC Rust ↔ JS    │     │ → Spacebot API │
└────────────────────────────────┘     └────────────────┘
```
