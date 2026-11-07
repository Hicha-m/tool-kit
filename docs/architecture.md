# Architecture

## Frontière de confiance

React affiche les outils et construit une demande typée. Les commandes Tauri valident de nouveau les paramètres. Le frontend ne reçoit aucun accès shell ni accès générique au système de fichiers. Les dialogues natifs sélectionnent des chemins ; le backend contrôle leur existence et les formats autorisés. Aucun argument CLI arbitraire n’est accepté.

`lib.rs` expose le cœur natif indépendamment de Tauri ; `queue.rs` gère les lots et l’exclusion installation/exécution. `model.rs` définit les demandes et instantanés ; `runner.rs` traduit les opérations en arguments ; `engines.rs` gère les distributions ; `store.rs` conserve les résultats dans SQLite. L’enregistrement d’un lot est transactionnel : une erreur ne laisse pas une moitié de lot en file. Les entrées de la file restent en mémoire et ne sont pas reprises après redémarrage. Une seconde instance est refusée afin d’éviter deux workers simultanés.

## Contrat v1

Demande : `operation`, `inputs`, `destination` (dossier), `format`, `pages`, `width`, `quality`, `track`. Les lots sont divisés en tâches individuelles, sauf la fusion PDF. Les paramètres sans rapport avec l’opération sont ignorés et ne deviennent jamais des arguments libres.

États : `queued → running → succeeded | failed | cancelled`. Au redémarrage : les tâches inachevées deviennent `interrupted`. Instantané : `id`, `operation`, `status`, date Unix en secondes, chemin de résultat éventuel et message d’erreur éventuel.

Commandes : `list_tasks`, `list_engines`, `enqueue_task`, `cancel_task`, `clear_history`, `open_result`, `install_engine`, `stop_all`. Événement `tasks-changed` après transition ; l’interface utilise actuellement une synchronisation toutes les 1,5 seconde. Progression indéterminée dans ce premier socle.

## Fichiers et processus

Les sources sont uniquement lues. La sortie est un fichier temporaire dans la destination ; `persist_noclobber` évite les collisions même si un autre processus crée le même nom. Les noms finaux utilisent `_toolkit`, puis un suffixe numérique. Une sortie réussie est conservée même si un lot échoue ensuite.

Chaque moteur est lancé dans un groupe de processus, avec entrée standard fermée, sans shell. L’annulation tue ce groupe et attend sa fin. Les fichiers temporaires de sortie sont enregistrés avant exécution pour récupération au démarrage. Un garde Rust nettoie aussi leur enregistrement après échec ou annulation. SQLite attend jusqu’à cinq secondes en cas de verrou concurrent. Les erreurs moteur affichées sont génériques afin de ne pas conserver le contenu des documents dans les journaux. ImageMagick reçoit une politique limitant les codeurs et interdisant les délégués ; FFmpeg limite les protocoles d’entrée à `file,pipe`.

## Ajouter un module

1. Décrire ses opérations et paramètres dans le registre versionné.
2. Ajouter les entrées d’interface et mettre à jour le test de concordance.
3. Implémenter validation, arguments structurés, annulation et validation de sortie dans Rust.
4. Qualifier chaque distribution de moteur et ses dépendances.
5. Ajouter les fichiers représentatifs et tests fonctionnels aux deux plateformes.

Les futurs adaptateurs HTTP locaux devront respecter le même cycle de tâche, sans exposer de CLI dans l’interface. Les plugins tiers restent exclus.

## Travaux restant avant stabilisation

Génération automatique des types TypeScript depuis Rust, progression FFmpeg structurée, erreurs avec codes stables, découverte dynamique complète des capacités, vérification détaillée des sorties multimédias et tests d’intégration avec les moteurs réels. Le registre JSON est versionné ; les types frontend sont encore maintenus manuellement.

Le build `--no-default-features --lib` n’embarque pas les bibliothèques graphiques. Les tests natifs injectent les chemins des exécutables via une fonction Rust privée ; aucune commande IPC ni variable d’environnement de production ne contourne le catalogue qualifié.
