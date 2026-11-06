# Livraison et critères d’acceptation

## État initial

Le code a été créé dans un environnement sans Rust, sans GTK/WebKit et sans accès réseau opérationnel. Seuls les tests Node sans dépendances ont pu être exécutés. La CI ci-jointe doit compiler l’interface et le backend sur les deux plateformes. Elle ne remplace pas une recette sur Windows 11 : le runner `windows-latest` utilise Windows Server.

## Avant une première release utilisable

- [ ] Résoudre les dépendances, générer et commiter `package-lock.json` et `Cargo.lock`, puis utiliser `npm ci` et Cargo `--locked` en CI.
- [ ] Faire réussir compilation et tests sur Windows et Ubuntu.
- [ ] Qualifier les six distributions moteur/plateforme et publier les notices et sources requises.
- [ ] Compléter les contrats générés et la découverte des modules ; traiter les codes d’erreur et avertissements.
- [ ] Afficher taille, licence et version avant confirmation d’installation ; ajouter progression et annulation du téléchargement.
- [ ] Ajouter progression FFmpeg via `-progress`, sondage et sélection explicite des pistes audio.
- [ ] Ajouter glisser-déposer, gestion du focus et confinement clavier de la modale.
- [ ] Tester chaque conversion avec vérification du contenu produit, pas uniquement du code de retour.
- [ ] Tester PDF chiffrés/invalides, accents, espaces, très grands fichiers, permissions, disque plein, collisions et lot partiellement invalide.
- [ ] Tester annulation, fermeture, redémarrage et préservation des sources.
- [ ] Tester téléchargement interrompu, empreinte incorrecte, extraction malveillante, mise à niveau échouée et usage hors ligne.
- [ ] Produire NSIS et `.deb`, tester sur machines propres Windows 11 / Ubuntu 24.04 sans outils de développement.
- [ ] Vérifier les dépendances WebView2 / WebKitGTK et inclure toutes les licences.

Pas de publication automatique dans le workflow initial. Les installateurs générés par la CI sont des artefacts de test. La signature Windows nécessite un certificat qui n’est pas configuré.
