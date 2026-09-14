# UNI - Analyse approfondie de l'application

Date: 2026-09-14. Périmètre: dépôt `~/projects/active/apps/uni`, commits `33e501b..5feddb5` (29 commits), version développeur v0.18. Méthode: lecture de tout le code, `cargo clippy`, exécution de la suite complète, reproduction active de chaque faille soupçonnée (deux ont été confirmées et corrigées pendant cette analyse), comparaison avec SPEC-001, le PRD v0.1 et le blueprint, données réelles de l'étude à 7 runs d'agent.

---

## 1. Chiffres

| Mesure | Valeur |
|---|---|
| Core (hors tests) | ~1 300 lignes Rust utiles, 2 175 au total, 6 crates |
| Tests | 33 verts (parser 6, decision 10 dont 2 proptests, CLI 17 dont e2e git/contenu/sécurité/golden) |
| Dépendances runtime | 23 directes, 109 paquets lockés (serde, clap, toml, chrono, sha2, anyhow; zéro tokio, zéro HTTP client) |
| Clippy | 0 erreur, 9 warnings cosmétiques |
| Dogfood | 5 stacks (Rust x2, Python, Node, forbid) ACCEPTED en release |
| Étude agent réel | n=7, 7 "DONE" auto-rapportés, 0 ACCEPTED, 0 faux acceptés, accord humain 7/7 sur les blocages |

## 2. Architecture

Le graphe de dépendances est propre et acyclique:

```
uni-parser -> uni-ir -> uni-evidence -> uni-verify
                                   \-> uni-decision -> uni-cli (binaire, fin)
```

- **Seam haut respecté**: `assure()`/`evaluate()` sont les portes d'entrée publiques; parser, store et verifiers restent des détails internes. C'est le choix de l'ADR-001 et il tient.
- **Découplage moteur/policy**: `evaluate` = table de vérité pure, `apply_policy` = correctif déterministe post-table. La policy ne peut qu durcir ou escalader, jamais affaiblir. Bon design, conforme au principe "verifiers prove, policies decide admissibility".
- **Un seul point de contact avec l'exécution** (`run_shell`, privatisé en v0.18): tout passe par le registre. La frontière de confiance contrat-non-fiable / registre-fiable est réellement implémentée, testée, et elle tient.
- **Dette connue**: `uni-cli/main.rs` concentre 10 commandes (693 lignes). Acceptable à v0, à découper en `commands/` avant la phase cloud.

## 3. Conformité SPEC-001 / PRD v0.1

| Exigence | Statut | Preuve |
|---|---|---|
| FR-001 init | OK | `.uni/{config.toml,contracts,evidence,decisions,artifacts}` |
| FR-002 parse + diagnostics | OK | fixtures d'erreur testées (ligne + suggestion) |
| FR-003 compile IR | OK | JSON canonique + JSON Schema |
| FR-004 model claim | OK | id/kind/required/critical/ensure |
| FR-005..007 verifiers shell/test | OK | via commande + expect; runners par registre |
| FR-008 Playwright | **Partiel** | passe comme commande, pas de `BrowserEvidence` structurée (noms de tests, durée par test) |
| FR-009..010 git binding + hash | OK | commit+dirty+sha256, testé e2e |
| FR-011..012 store + états | OK | Valid/Invalid/Stale (UNVERIFIED/REVOKED non implémentés, hors v0.1) |
| FR-013 invalidation | OK x2 | commit ET contenu surveillé (`files` globs + `artifact_hash`) |
| FR-014 décision | OK | + ESCALATED au-delà du PRD |
| FR-015 explain | OK | rapport sectionné + détail par claim |
| FR-016 `--json` | OK | global, et `report` stable-octets |
| FR-017..018 GitHub | **Partiel** | workflow OK; `action.yml` non consommable par un repo externe (chemin binaire relatif au dépôt action, jamais buildé) |
| NFR perf (compile <100ms, CLI <300ms) | OK en pratique | mesuré subjectivement, pas de bench officiel |
| NFR Windows | **Non couvert** | `sh -c` partout, `timeout` externe optionnel, zéro CI Windows |
| §11 déterminisme | OK | 2 proptests (idempotence engine + policy) |
| §10 sécurité commandes | OK | 3 tests sécurité, dont 2 nouveaux |
| Hors-scope §14 respecté | OK | zéro orchestration, registry, MCP, A2A, cloud dans le core |

Champs déclaratifs inertes (honnête): `acceptance.require_verified`/`allow_critical_failures` et `constraints` (REQUIRE) sont parsés, stockés, mais **lus par personne dans le moteur**. Le comportement par défaut du moteur coïncide avec la valeur par défaut, donc pas de divergence visible; mais un contrat qui écrirait autre chose ne changerait rien. À câbler ou à retirer.

## 4. Failles trouvées pendant cette analyse (2 confirmées, corrigées le même commit)

### F1 - Réutilisation d'evidence entre contrats (sévérité: critique, corrigée en v0.18)

Le cache d'evidence était indexé par `claim_id` seul. Deux contrats partageant un id de claim se volaient leur preuve. Reproduction exacte exécutée ici: contrat c1 (`x USING ok`, `ok=true`) accepté; puis c2 (`x USING bad`, `bad=false`) **accepté sans exécuter `false`**, en relisant le JSON de c1. Le verdict d'assurance d'un contrat pouvait donc être prouvé par le travail d'un autre, ce qui inverse exactement la promesse du produit ("never trust done, verify the outcome").

Fix: `fingerprint = sha256(verifier_ref|run|expect|expect_not|files)[..12]` stocké dans l'evidence et intégré au nom de fichier (`claim__fp.json`), plus contrôle à la lecture. Le cache est désormais isolé par spécification de vérificateur. Test de régression: `evidence_cannot_cross_contract_boundary`.

### F2 - Faux négatif `expect`/`expect_not` au-delà de 2000 caractères (sévérité: haute, corrigée)

Les matchers s'evaluaient sur `output_excerpt`, tronqué à 2000 caractères. Un contenu interdit placé après la fenêtre (ex: un `grep` long dont l'ultima ligne porte `EVIL`) donnait une evidence `Valid` alors que la prohibition était violée. C'est le trou béant de l'exemple FORBID lui-même.

Fix: l'exécution complète est vérifiée sur la sortie intégrale (`full_output`), l'extrait tronqué ne sert plus qu'à l'affichage. Test: `expect_not_beyond_excerpt_window_invalidates` (marker caché après 3000 `x`).

Les deux failles avaient le même racine: prouver une propriété sur une représentation partielle. Le correctif les ferme par le même principe: l'identité complète de la vérification entre dans la clé, et la propriété se vérifie sur le contenu intégral.

## 5. Limites restantes, par criticité décroissante

1. **Pas de vérification indépendante (A3/A4)**. `assurance_level` est un mapping de la décision (A0/A1/A2), pas une mesure. La règle `executor != verifier` et les attestations crypto (Sigstore/SLSA) n'existent nulle part. C'est le cœur de la catégorie "outcome assurance" à terme; pour l'instant UNI prouve "un exécuteur de confiance a vu", pas "un tiers a revu". Le `REQUIRE executor != verifier` parse mais n'est pas appliqué (voir §3).
2. **Le contrat couple la preuve au NOM du test.** Les verificateurs attendent `cargo test clamp_upper_works -- --exact`. Un agent qui répare bien mais nomme le test autrement produit `EVIDENCE_REQUIRED` (constaté dans l'étude: 50% de couverture evidence, tous les blocages explicables). Deux options à trancher en v0.2: guidage des noms de tests dans l'issue, ou sélecteur flou par description.
3. **Le registre est du RCE par design.** `expect` ne protège pas d'un `run` malveillant: la confiance est reportée sur `.uni/config.toml` (fichier du repo, versionné, reviewable). Manque: hook de review des changements de registre (un diff bot qui bloque toute modification de `[verifiers]` est la vraie porte), et `uni doctor` ne signale même pas que le registre a changé depuis le dernier commit.
4. **Portabilité**: `sh -c` et `which timeout` sont POSIX-only; sans coreutils le `timeout` du registre est silencieusement ignoré (risque: vérificateur bloquant qui retient un CI). Un `wait-timeout` crate (petit, sans tokio) ou thread+kill résoudrait les deux. Windows hors sujet v0.1 mais le PRD le listait; à retirer du PRD ou à tester.
5. **`action.yml` non publié**: le chemin `${{ github.action_path }}/../../target/release/uni` ne marche que si l'action vit dans le repo qui a buildé. Pour les consommateurs: soit un release binaire téléchargé par l'action, soit l'action qui build. Le workflow interne, lui, est correct.
6. **Hygiène runtime**: `events.jsonl` sans rotation; `last.json`/evidence sans verrou (deux `uni verify` parallèles peuvent écrire un JSON croisé; `save_json` écrit directement, pas de tmp+rename atomique); `.gitignore` du dépôt racine ignore `evidence/decisions/artifacts` mais pas `contracts/`, d'où les artefacts de candidat vus dans `git status`.
7. **Zéro test unitaire dans `uni-ir`, `uni-evidence`, `uni-verify`**: ils ne sont couverts que par les tests CLI/intégration. Les pièges de cette analyse (clés de cache, fenêtres de sortie) vivaient justement dans ces crates. Ajouter des unit tests ciblés (fingerprint, artifact_hash sur symlink? - note: `std::fs::read` suit les liens, un `files` glob peut hacher hors workspace par symlink, mineur car le registre est déjà la porte de sortie).
8. **Doublon mineur**: le mapping décision->assurance existe en 3 exemplaires (`assurance_level_x`, l'événement `DecisionIssued`, `report`). À factoriser avant d'ajouter A3/A4.

## 6. Ce qui est solide

- **Le noyau décisionnel est juste et prouvé comme tel**: table exhaustive, proptest d'idempotence, et les deux bugs de sécurité de cette session ont été trouvés par un raisonnement d'adversaire puis bouchés avec test de régression. C'est le comportement attendu d'un outil d'assurance.
- **L'invalidation est la vraie valeur différenciante**: commit + dirty + contenu surveillé, testée e2e dans un git temporaire. Peu d'outils "agent QA" font ça; la plupart ne regardent que le présent.
- **L'incrémentalité est réelle** (cache hit visible dans `events`), et borne le coût de re-vérification sans sacrifier la fraicheur (les deux failles venaient de là, elles sont fermées).
- **La stack est minimale**: 6 crates, pas d'async, pas de network, 23 dépendances directes, build release silencieux. Un projet qui peut tenir un `git clone + uni verify` en 60 secondes, promesse tenue (quickstart vérifié en manuel).
- **Le wedge produit est validé sur données**: l'étude n=7 (agent gratuit, 7 faux "DONE", 0 faux acceptés) n'est pas un benchmark de modèle mais bien une démonstration du mécanisme: le self-report ment, l'evidence parle. Le rapport d'étude est honnête sur ses limites.
- **La boucle outil-outil fonctionne**: Spec Kit (runtime réel installé) -> UNI (import + candidats) -> Matt skills (tickets `.scratch`) -> UNI dogfood son propre dépôt à chaque commit (règle de constitution tenue depuis le premier slice).

## 7. Note de maturité

| Dimension | /10 | Commentaire |
|---|---|---|
| Cohérence architecture/intention | 9 | Le seam haut et la frontière de confiance sont tenus partout |
| Sécurité du modèle de preuve | 7 | F1/F2 corrigés; reste l'absence d'attestation tierce (A3/A4) |
| Couverture de tests | 8 | 33 tests, proptests, e2e git; manque l'unit dans 3 crates basses |
| Portabilité / ops | 5 | POSIX-only, timeout externe, rotation/locks absents |
| Intégrations | 4 | workflow interne OK; action externe, OPA binaire, Playwright structuré, OTel export: à l'état de câble armé |
| Documentation | 9 | set complet PRD §24 + cette analyse; README à jour capability map |
| Différenciation produit | 8 | l'invalidation d'evidence nulle part ailleurs; risque: category creation lente |

Score global: **prototype convaincant, produit developer-preview crédible**. Rien ne s'oppose à un tag `v0.1.0` après: (a) suppression ou câblage des champs inertes (`acceptance`, `constraints`), (b) verrou + écriture atomique sur last.json, (c) unit tests `evidence`/`verify`. Ce sont des demi-journées.

## 8. Roadmap recommandée (ordre = valeur par effort)

1. v0.19: hygiène des preuves (tmp+rename, lock flock, rotation events, unit tests bas crates) - protège ce qui vient d'être corrigé.
2. v0.19: câbler `acceptance`/`REQUIRE` dans le moteur ou retirer les champs (une spec ne doit rien promettre d'inerte).
3. v0.20: A3 - séparateur executor/verifier démontrable (règle `executor != verifier` appliquée, champ producteur vs actor dans l'evidence).
4. v0.20: action GitHub publique avec release binaire + cross-compiles (macOS/Linux/Windows).
5. v1.1: sélecteur claim->tests flou, puis study n=50 avec un agent capable (débloque quand l'accès premium/model fort est réglé).
6. Ensuite seulement: cloud org/dashboards (cost per accepted outcome), `BrowserEvidence` structurée, OTel exporter.
