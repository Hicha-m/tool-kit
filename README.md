# Tool-kit

Une boîte à outils locale et modulaire, en français, pour Windows 11 x64 et Ubuntu 24.04 x64.

## État du projet

**Socle initial en développement, pas une V1 prête à distribuer.** Interface React, hôte Tauri 2, file séquentielle Rust, historique SQLite et adaptateurs qpdf / FFmpeg / ImageMagick sont présents. Les archives de moteurs ne sont pas encore qualifiées : le catalogue reste volontairement vide et les traitements sont donc désactivés. Il n’y a aucune conversion simulée.

Le réseau de l’environnement de création est indisponible ; Rust et les dépendances GTK/WebKit sont absents. Les tests Node sont exécutables sans installation ; la compilation React/Tauri et les recettes natives doivent encore réussir en CI. Aucun installateur validé n’est fourni à ce stade.

## Développement

Installer Node.js 22 LTS, npm et Rust stable. Sous Ubuntu 24.04 :

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf
npm install
npm test
npm run tauri dev
```

Sous Windows : installer Visual Studio Build Tools (C++), Rust MSVC et WebView2, puis exécuter les trois commandes npm ci-dessus.

```bash
npm run dev        # aperçu de l’interface dans un navigateur, sans traitement natif
npm run build      # vérification TypeScript et compilation de l’interface
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build -- --bundles deb  # Ubuntu
npm run tauri build -- --bundles nsis # Windows
```

Les lockfiles ne sont pas inventés : ils devront être générés et commités après une première résolution réseau réussie. La CI utilise provisoirement `npm install`.

## Fonctions prévues dans ce socle

- PDF : fusion et extraction de pages ; refus des documents chiffrés.
- Audio : MP3 192 kbit/s, WAV PCM 16 bits, FLAC.
- Vidéo : MP4 H.264/AAC logiciel ; extraction d’une piste audio.
- Images statiques : JPEG, PNG, WebP ; redimensionnement proportionnel sans agrandissement.
- Recherche, favoris locaux, lots séquentiels, annulation, historique des 100 derniers résultats.

Aucun compte, télémétrie ou transfert de document. Les téléchargements de moteurs ne démarrent que sur action explicite et depuis le catalogue embarqué. L’historique stocke les chemins de sortie, pas les contenus ni les demandes de traitement.

Voir [l’architecture](docs/architecture.md), [la qualification des moteurs](docs/engines.md) et [la checklist de livraison](docs/release.md).

## Licence

Code applicatif : GPL-3.0-only, voir [LICENSE](LICENSE). Les moteurs sont des projets distincts avec leurs propres obligations. Aucun binaire moteur n’est actuellement redistribué.
