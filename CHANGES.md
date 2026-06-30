# Changements du fork

## v0.5.0-fork.1 — 2026-06-30

### Proxy Umbrel intégré à l'app desktop

Le code Rust de l'app desktop intègre désormais un serveur proxy HTTP qui résout
automatiquement l'authentification Umbrel (Caddy reverse proxy). Plus besoin de
lancer un processus séparé.

**Changements :**

- Nouveau module `desktop/src-tauri/src/proxy.rs` — serveur HTTP (axum) qui
  forwarde les requêtes vers la cible configurée
- Détection et résolution automatique des redirections 302 de Caddy vers le
  portail d'auth Umbrel (port 2000)
- Cookie store géré par `reqwest` — session persistante automatiquement
- Le proxy écoute sur `127.0.0.1:19777`, le frontend l'utilise comme base API
- Support du streaming SSE pour le cortex chat et événements temps réel

**Frontend :**

- Nouveau champ "Umbrel Password" dans l'écran de connexion (visible pour les
  URL distantes uniquement)
- `getApiBase()` retourne automatiquement l'URL du proxy en mode desktop
- Health check routé via le proxy

**Build :**

- Suppression du sidecar `externalBin` (plus de binaire serveur embarqué)
- Script `bundle-sidecar.sh` désactivé
- Nouvelles dépendances Rust : `axum`, `reqwest` (cookie, json, stream),
  `tower-http` (cors), `tokio` (full), `futures`, `url`, `bytes`

### Fichiers modifiés

```
desktop/src-tauri/src/proxy.rs          (nouveau)
desktop/src-tauri/src/main.rs           (+proxy, nouvelles commandes Tauri)
desktop/src-tauri/Cargo.toml            (+dépendances)
desktop/package.json                    (sidecar désactivé)
desktop/src-tauri/tauri.conf.json       (externalBin supprimé)
interface/src/api/client.ts             (proxy URL en desktop)
interface/src/platform.ts               (PROXY_PORT)
interface/src/hooks/useServer.tsx        (health check via proxy)
interface/src/components/ConnectionScreen.tsx (champ Umbrel password)
```
