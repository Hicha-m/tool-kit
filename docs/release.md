# Livraison et critères d’acceptation

## État initial

Le socle initial a compilé et produit ses paquets de test en CI Windows/Ubuntu. La suite ajoutée contrôle maintenant les conversions avec de vrais moteurs ; voir `regression.md` pour les résultats et les limites. Les dépendances graphiques restent absentes de l’environnement local, où le cœur est testé sans Tauri. Elle ne remplace pas une recette sur Windows 11 : le runner `windows-latest` utilise Windows Server.

## Avant une première release utilisable

- [x] Dépendances résolues, lockfiles générés et versionnés ; utilisation de `npm ci` et Cargo `--locked` en CI.
- [ ] Faire réussir compilation et tests sur Windows et Ubuntu.
- [ ] Qualifier les six distributions moteur/plateforme et publier les notices et sources requises.
- [ ] Compléter les contrats générés et la découverte des modules ; traiter les codes d’erreur et avertissements.
- [ ] Afficher taille, licence et version avant confirmation d’installation ; ajouter progression et annulation du téléchargement.
- [ ] Ajouter progression FFmpeg via `-progress`, sondage et sélection explicite des pistes audio.
- [ ] Ajouter glisser-déposer, gestion du focus et confinement clavier de la modale.
- [x] Tests des sept opérations sur fichiers générés, avec vérification du contenu produit. À répéter lors de la qualification des distributions de production.
- [ ] Tester PDF chiffrés/invalides, accents, espaces, très grands fichiers, permissions, disque plein, collisions et lot partiellement invalide.
- [ ] Tester annulation, fermeture, redémarrage et préservation des sources.
- [ ] Tester téléchargement interrompu, empreinte incorrecte, extraction malveillante, mise à niveau échouée et usage hors ligne.
- [ ] Produire NSIS et `.deb`, tester sur machines propres Windows 11 / Ubuntu 24.04 sans outils de développement.
- [ ] Vérifier les dépendances WebView2 / WebKitGTK et inclure toutes les licences.

Pas de publication automatique dans le workflow initial. Les installateurs générés par la CI sont des artefacts de test. La signature Windows nécessite un certificat qui n’est pas configuré.
