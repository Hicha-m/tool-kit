# Tests fonctionnels et régression

## Exécution du 6 novembre 2026

Le cœur Rust et l’interface ont été testés localement. La CI exécute la même suite sur Ubuntu 24.04 et Windows ; ses rapports et les versions exactes sont archivés avec chaque exécution. Aucun test de moteur ne passe silencieusement si un exécutable est absent : la configuration ou le test échoue.

| Fonction | Vérification réelle de la sortie |
|---|---|
| Fusion PDF | Ordre réordonné de trois pages et contenu textuel de chaque page |
| Extraction PDF | Sélection `1-2,4`, contenu et ordre ; pages inexistantes refusées |
| Conversion audio | MP3/WAV/FLAC, codec, durée et fréquence d’un signal connu ; égalité des échantillons pour les formats sans perte |
| Conversion vidéo | Décodage des cinq images, H.264/AAC, dimensions ; vidéo sans audio ; dimensions impaires complétées d’un pixel pour H.264 |
| Extraction audio | Seconde piste à 880 Hz, trois formats ; piste absente et vidéo silencieuse refusées |
| Conversion d’images | Neuf paires JPEG/PNG/WebP, dimensions et pixels ; transparence sur blanc en JPEG ; orientation EXIF |
| Redimensionnement | Rapport 2:1 conservé, dimensions exactes et absence d’agrandissement |

Chaque appel d’adaptateur compare les sources octet par octet avant/après. Les tests vérifient l’absence de sortie temporaire et d’enregistrement temporaire résiduel après réussite ou échec.

### Régressions transversales

- Fichiers invalides dans les six parcours à entrée unique ; PDF chiffré et WebP animé refusés.
- Chemins avec espaces et accents ; collision sans écrasement ; destination absente et, sous Unix, dossier non inscriptible.
- Annulation avant traitement et interruption d’un vrai processus FFmpeg en moins de cinq secondes.
- File FIFO, annulation indépendante, limite de 200 tâches, exclusion des installations, fermeture et transaction de lot annulée sur erreur SQLite injectée.
- Récupération après interruption ; historique limité à 100 résultats sans suppression des tâches actives.
- ZIP valide, mauvaise empreinte, téléchargement tronqué/interrompu/trop grand, taille d’extraction excessive, traversée de chemin, lien symbolique et conservation de l’ancienne version.
- Interface : recherche sans accents, favoris, mémoire locale corrompue/refusée, contrats envoyés par les sept outils, réordonnancement PDF, réglages, absence de moteur, erreurs, annulation et ouverture/effacement de l’historique.

### Régressions corrigées

- Une fusion avec un seul PDF reste désactivée dans l’interface.
- Les réglages de largeur reviennent à leur valeur par défaut à la réouverture.
- Le bouton de destination possède un nom accessible stable.
- Un lot est enregistré entièrement ou pas du tout ; les erreurs SQLite ne laissent pas un lot partiel.
- Les temporaires et leurs enregistrements sont nettoyés après erreur ou annulation.
- Les handles temporaires sont fermés avant l’appel aux CLI ; les chemins Windows canoniques sont normalisés avec `dunce` pour éviter leur interprétation incorrecte par qpdf. Un test Windows transmet explicitement des chemins d’entrée et de destination au format étendu.

## Reproduire

```bash
npm ci
npm test
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --lib
```

La dernière commande n’exécute que les tests rapides et signale explicitement les tests natifs ignorés. Pour **toute** la suite, définir quatre chemins absolus vers `TOOLKIT_QPDF`, `TOOLKIT_FFMPEG`, `TOOLKIT_FFPROBE`, `TOOLKIT_MAGICK`, puis :

```bash
python scripts/run-native-tests.py --headless
```

Le mode `--headless` utilise exactement le même runner de production, sans les dépendances graphiques. En CI, `scripts/test-engines.py` installe/résout les vrais moteurs puis `run-native-tests.py` exécute également les tests de l’hôte bureau. Les logs sont conservés dans les artefacts CI, avec `versions.json`. Les variables de test ne sont jamais consultées par le runner de production.

Versions locales : qpdf 12.4.2 (archive officielle SHA-256 `db367d897829f22c4198ce1094143c9d467bd6ee7dfabc44ba6f02056b24f8b1`), FFmpeg 7.1.5 Debian, ImageMagick 7.1.1-43. Ubuntu CI utilise FFmpeg/qpdf de la distribution et ImageMagick 7.1.2-35 officiel avec empreinte figée. Windows CI utilise les distributions Chocolatey et archive leurs versions effectives.

## Limites explicites

Ces résultats valident les adaptateurs avec les exécutables testés, **pas encore les six paquets de production**, dont le catalogue reste vide. L’interface est testée sous jsdom avec le pont Tauri simulé ; les dialogues natifs, l’ouverture réelle du fichier et le rendu visuel de l’application installée doivent encore être vérifiés sur machine propre.

L’installateur est testé au niveau des flux et archives : aucune release moteur réelle n’a encore été téléchargée/activée par l’application. La panne de stockage est injectée dans SQLite ; un disque physiquement plein n’a pas été reproduit. Le test d’annulation contrôle FFmpeg, sans preuve spécifique de terminaison de petits-enfants sur chaque système. La signature Windows, la progression chiffrée et la qualification des licences restent à terminer.
