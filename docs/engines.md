# Qualification des moteurs

## Candidats repérés le 5 novembre 2026

| Moteur | Distribution candidate | Windows x64 | Linux x64 | Statut |
|---|---|---|---|---|
| qpdf | officiel 12.4.2 | mingw64.zip | bin-linux-x86_64.zip | À qualifier |
| FFmpeg / ffprobe | BtbN, branche 8.1, GPL statique | ZIP | tar.xz | À qualifier et figer |
| ImageMagick | officiel 7.1.2-35 | portable Q16 x64 | AppImage extraite | À qualifier |

Ces observations ne constituent pas un audit des licences, codecs ou dépendances. Les étoiles et la maintenance restent à relever auprès des dépôts officiels. Aucun chiffre non vérifié n’est publié.

## Procédure obligatoire

- Télécharger l’archive depuis une source identifiée et figer son identité/version effective ; enregistrer la date, l’empreinte, l’URL, la recette de compilation disponible et les dépendances.
- Vérifier licences des moteurs ET des dépendances, codecs réellement présents et disponibilité des sources correspondantes. Ne pas utiliser de build `nonfree`.
- Exécuter les fonctions prévues sur Windows 11 et Ubuntu 24.04 propres, sans outils système supplémentaires.
- Normaliser le paquet vérifié en ZIP contenant le moteur portable, ses bibliothèques, configurations, licences et notices. L’extraction de l’AppImage et le repaquetage sont faits pendant cette qualification, pas chez l’utilisateur. Conserver la provenance et l’empreinte de l’archive d’origine.
- Publier le ZIP normalisé et les sources/notices requises dans une release moteur dédiée de `Hicha-m/tool-kit`. Ne pas écraser une archive déjà publiée.
- Ajouter l’entrée correspondante à `src-tauri/catalog/engines.json`. L’empreinte porte sur le ZIP effectivement téléchargé. Ne jamais remplacer une empreinte pour faire passer un téléchargement inattendu.

## Catalogue v1

`schemaVersion: 1`, liste `artifacts`. Chaque entrée contient :

```json
{
  "id": "ffmpeg",
  "version": "VERSION-QUALIFIEE",
  "platform": "windows-x86_64",
  "url": "https://github.com/Hicha-m/tool-kit/releases/download/TAG/ARCHIVE.zip",
  "sha256": "EMPREINTE-SHA256-REELLE",
  "size": 1,
  "unpackedLimit": 1,
  "executable": "bin/ffmpeg.exe",
  "probe": "bin/ffprobe.exe",
  "license": "IDENTIFIANT-LICENCE-VERIFIE"
}
```

Cet exemple est un schéma illustratif, pas une entrée installable. `size` est la taille exacte du ZIP ; `unpackedLimit` borne son extraction. Plateformes autorisées en V1 : `windows-x86_64`, `linux-x86_64`. Pour qpdf/ImageMagick, `probe` vaut `null`. Les anciennes installations restent sur disque après une mise à niveau ; le catalogue sélectionne la version active. Une réinstallation identique est refusée dans ce socle.

## Limites actuelles

Pas encore de progression/annulation du téléchargement ni de test détaillé des capacités avant activation : seule la commande de version est lancée. Ces validations doivent être complétées avant de remplir le catalogue de production. Aucun moteur ne doit être activé tant que la recette n’est pas réussie.
