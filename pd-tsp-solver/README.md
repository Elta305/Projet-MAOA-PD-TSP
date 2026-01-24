# PD-TSP Solver

Solveur pour le problème du **Pickup and Delivery Traveling Salesman Problem (PD-TSP)**.

## Description

Le PD-TSP est une variante du TSP classique où :
- Le véhicule commence au dépôt avec une charge initiale
- Certains nœuds sont des **pickups** (demande négative = le véhicule charge des biens)
- D'autres sont des **deliveries** (demande positive = le véhicule décharge des biens)
- La charge du véhicule doit rester dans l'intervalle `[0, capacité]` tout au long du parcours
- Le véhicule retourne au dépôt avec une charge finale de 0

## Format des instances (TS2004)

### Deux formats différents

Le solveur gère automatiquement **deux types d'instances** :

#### Type 1 : Instances avec dépôt dupliqué (n20mos*, n30mos*)
- Le fichier contient `DIMENSION: n+1` nœuds
- Le **nœud 1** et le **nœud n+1** sont tous deux le **dépôt** (mêmes coordonnées, souvent 0,0)
- Le nœud 1 représente le départ (avec une demande initiale)
- Le nœud n+1 représente le retour (avec la demande finale pour équilibrer)
- Le solveur détecte automatiquement cette duplication et travaille avec **n nœuds distincts**

**Exemple** : `n20mosA.tsp`
```
DIMENSION: 21
NODE_COORD_SECTION
1   0.0000   0.0000    # Dépôt de départ
2 220.0000 -461.0000
...
20 118.0000 112.0000
21   0.0000   0.0000   # Dépôt de retour (même position que nœud 1)

DEMAND_SECTION
1 37                   # Demande au départ
...
21 -44                 # Demande au retour (équilibre à 0)
```
→ Le solveur traite **20 nœuds** (1 dépôt + 19 clients)

#### Type 2 : Instances sans duplication (n20q*, n30q*)
- Le fichier contient `DIMENSION: n` nœuds (pas de duplication)
- Seul le **nœud 1** est le dépôt
- Le véhicule retourne simplement au nœud 1 à la fin
- Le solveur calcule automatiquement la demande de retour nécessaire

**Exemple** : `n20q10A.tsp`
```
DIMENSION: 20         # Pas de nœud 21
NODE_COORD_SECTION
1   0.0000   0.0000   # Dépôt (pas de duplication)
2 220.0000 -461.0000
...
20 118.0000 112.0000  # Dernier client (pas le dépôt)

DEMAND_SECTION
1 -7
...
20 4
```
→ Le solveur traite **20 nœuds** (1 dépôt + 19 clients)
→ La demande de retour est calculée automatiquement pour équilibrer à 0

### Détection automatique

Le solveur **détecte automatiquement** le type d'instance en comparant les coordonnées du premier et du dernier nœud :
- Si identiques (distance < 1e-6) → Type 1 avec duplication
- Si différents → Type 2 sans duplication

⚠️ **Aucune action requise** : le code gère les deux formats de manière transparente.

### Format des demandes
- **Demande positive** = pickup (augmente la charge)
- **Demande négative** = delivery (diminue la charge)
- La somme de toutes les demandes (nœuds 1 à n+1) = 0 (conservation)

## Installation

### Prérequis
- Rust 1.70+ : https://rustup.rs/
- (Optionnel) Gurobi 12.0+ pour le solveur exact

### Compilation
```bash
cd pd-tsp-solver
cargo build --release
```

Le binaire sera dans `target/release/pd-tsp-solver.exe`

## Utilisation

### Commande de base
```bash
cargo run --release -- solve -i <fichier_instance> -a <algorithme>
```

### Options disponibles
- `-i, --instance <FILE>` : Chemin vers le fichier d'instance (requis)
- `-a, --algorithm <ALGO>` : Algorithme à utiliser (défaut: hybrid)
- `-v, --verbose` : Affichage détaillé (statistiques de l'instance, profil de charge)
- `-t, --time-limit <SEC>` : Limite de temps en secondes (défaut: 60)
- `-s, --seed <NUM>` : Graine aléatoire pour la reproductibilité (défaut: 42)
- `-o, --output <FILE>` : Sauvegarder la solution dans un fichier
- `--visualize` : Générer une visualisation SVG

### Exemples
```bash
# Tester l'algorithme Greedy sur une instance
cargo run --release -- solve -i ../Datasets/TS2004t2/n20mosA.tsp -a greedy

# Avec affichage détaillé du profil de charge
cargo run --release -- solve -i ../Datasets/TS2004t2/n20mosA.tsp -a greedy -v

# Sauvegarder la solution
cargo run --release -- solve -i ../Datasets/TS2004t2/n20mosA.tsp -a vnd -o solution.json

# Avec visualisation SVG
cargo run --release -- solve -i ../Datasets/TS2004t2/n20mosA.tsp -a hybrid --visualize
```

## Algorithmes disponibles

### Heuristiques constructives (gloutonnes)

| Algorithme | Commande | Description |
|------------|----------|-------------|
| **Nearest Neighbor** | `nn` | Plus proche voisin avec contraintes de capacité |
| **Greedy Insertion** | `greedy` | Insertion gloutonne (minimise le coût d'insertion) |
| **Savings (Clarke-Wright)** | `savings` | Algorithme d'économies classique adapté au PD-TSP |
| **Sweep** | `sweep` | Balayage angulaire depuis le dépôt |
| **Regret Insertion** | `regret` | Insertion basée sur le regret (k=3) |
| **Cluster-First** | `cluster-first` | Clustering puis construction de routes |
| **Multi-Start** | `multi-start` | Essaie toutes les heuristiques et garde la meilleure |
| **ProfitDensity (Custom)** | `profit-density` | Heuristique basée sur le ratio profit/distance (robuste) |

### Recherches locales

| Algorithme | Commande | Description |
|------------|----------|-------------|
| **2-Opt** | `two-opt` | Recherche locale 2-opt |
| **VND** | `vnd` | Variable Neighborhood Descent |

### Métaheuristiques

| Algorithme | Commande | Description |
|------------|----------|-------------|
| **Simulated Annealing** | `sa` | Recuit simulé |
| **Tabu Search** | `tabu` | Recherche tabou |
| **ILS** | `ils` | Iterated Local Search |
| **Genetic Algorithm** | `ga` | Algorithme génétique |
| **Memetic Algorithm** | `memetic` | Algorithme mémétique (GA + recherche locale) |
| **Ant Colony** | `aco` | Optimisation par colonie de fourmis |
| **Max-Min Ant System** | `mmas` | MMAS variant de ACO |

### Autres

| Algorithme | Commande | Description |
|------------|----------|-------------|
| **Hybrid** | `hybrid` | Combinaison Multi-start + VND + ILS (recommandé) |
| **Exact (Gurobi)** | `exact` | Solveur exact MIP avec Gurobi |

## Tests complets

### Tester tous les algorithmes constructifs sur une instance
```bash
# Définir l'instance
$instance = "../Datasets/TS2004t2/n20mosA.tsp"

# Tester chaque heuristique
foreach ($algo in @("nn", "greedy", "savings", "sweep", "regret", "cluster-first", "profit-density", "multi-start")) {
    Write-Host "`n===== Test: $algo ====="
    cargo run --release -- solve -i $instance -a $algo
}
```

### Tester toutes les métaheuristiques
```bash
$instance = "../Datasets/TS2004t2/n20mosA.tsp"

foreach ($algo in @("two-opt", "vnd", "sa", "tabu", "ils", "ga", "memetic", "aco", "mmas", "hybrid")) {
    Write-Host "`n===== Test: $algo ====="
    cargo run --release -- solve -i $instance -a $algo -t 30
}
```

### Tester toutes les instances d'un répertoire
```bash
# Toutes les instances n20mos*
Get-ChildItem "../Datasets/TS2004t2/n20mos*.tsp" | ForEach-Object {
    Write-Host "`n===== Instance: $($_.Name) ====="
    cargo run --release -- solve -i $_.FullName -a hybrid
}

# Toutes les instances n20q10*
Get-ChildItem "../Datasets/TS2004t2/n20q10*.tsp" | ForEach-Object {
    Write-Host "`n===== Instance: $($_.Name) ====="
    cargo run --release -- solve -i $_.FullName -a greedy
}
```

### Benchmark complet (toutes instances, tous algos)
```bash
# Comparer plusieurs algorithmes sur toutes les instances n20mos
$algos = @("greedy", "vnd", "sa", "hybrid")
Get-ChildItem "../Datasets/TS2004t2/n20mos*.tsp" | ForEach-Object {
    $inst = $_.Name
    Write-Host "`n========== $inst =========="
    foreach ($algo in $algos) {
        Write-Host "  --- $algo ---"
        cargo run --release -- solve -i $_.FullName -a $algo | Select-String "(Cost:|Feasible:|Time:)"
    }
}
```

### Comparer les algorithmes avec statistiques
```bash
cargo run --release -- compare -i ../Datasets/TS2004t2/n20mosA.tsp -n 10
```
Cette commande exécute 10 fois chaque algorithme et affiche les statistiques (moyenne, écart-type, min, max).

## Structure de l'output

### Mode normal
```
Algorithm: GreedyInsertion
Cost: 4656.33
Feasible: true
Time: 0.0001s
```

### Mode verbose (`-v`)
```
Instance: n20mosA.tsp, 20 nodes, semilla 0
  Nodes: 20 (1 depot + 19 customers)
  Capacity: 44
  Pickup nodes: 8
  Delivery nodes: 8
  Total pickup load: 37
  Total delivery load: 44
  Avg distance: 495.07
  Max distance: 1033.27

Solving with Greedy algorithm...

========== Results ==========
Algorithm: GreedyInsertion
Cost: 4656.33
Feasible: true
Time: 0.0001s

Tour: [0, 16, 5, 1, 2, 7, 4, 13, 15, 6, 14, 18, 9, 17, 10, 11, 3, 12, 8, 19]
Load profile: [37, 34, 40, 37, 34, 29, 39, 33, 26, 29, 33, 26, 26, 30, 39, 43, 43, 40, 40, 44, 0]
Max load: 44
Min load: 0
```

**Notes importantes** :
- Le tour commence et finit au nœud 0 (dépôt)
- `Load profile` : charge après chaque visite (doit finir à 0)
- `Feasible: true` signifie que la charge reste toujours dans `[0, capacity]`

## Fonctions de coût

Le solveur supporte plusieurs fonctions de coût :

### 1. Distance (défaut)
Coût = somme des distances

### 2. Quadratique (heuristiques seulement)
Coût = `distance + (α × W + β × W^2)` où `W` est la charge quittant le nœud.
- Pénalise les déplacements avec charges élevées.
- NOTE: L'option quadratique est prise en charge par les heuristiques, mais n'est PAS supportée par le solveur exact Gurobi (le solveur exact ne supporte que le coût linéaire de distance). Use heuristics or linear-load cost for exact solving.

### 3. Linéaire-charge
Coût = `distance + (α × |W|)` où `W` est la charge quittant le nœud.
- Pénalisation linéaire de la charge (additive)
- Disponible via `instance.tour_cost_linear_load(tour, alpha)` or by using the CLI flag `--cost-function linear-load --alpha <value>`

## Heuristique personnalisée : ProfitDensity

**ProfitDensity** est notre nouvelle heuristique custom pour le PD-TSP, qui privilégie les nœuds avec un fort rapport profit/distance. Elle est conçue pour être robuste sous les deux modes de coût (distance linéaire et coût quadratique dépendant de la charge).

### Stratégie
1. Démarre au dépôt avec la charge initiale
2. À chaque étape, calcule un score profit/distance pour chaque candidat réalisable et choisit le meilleur
3. Préserve la faisabilité en rejetant les mouvements qui violeraient la capacité

### Utilisation
```bash
cargo run --release -- solve -i ../Datasets/TS2004t2/n20mosA.tsp -a profit-density -v
```

### Résultats
Sur les instances de test, ProfitDensity tend à sélectionner des nœuds à haute rentabilité relative, ce qui donne de bonnes performances pour les deux fonctions de coût.

## Instances disponibles

### Format TS2004t2
Le dossier `../Datasets/TS2004t2/` contient plusieurs familles d'instances :

#### n20mos* (instances de Mosheiov, 20 nœuds)
- `n20mosA.tsp` à `n20mosJ.tsp` (10 instances)
- Générées aléatoirement avec différentes graines

#### n20q* (instances avec différentes capacités, 20 nœuds)
- `n20q10*.tsp` : capacité 10
- `n20q15*.tsp` : capacité 15
- `n20q20*.tsp` : capacité 20
- `n20q25*.tsp` : capacité 25
- ... jusqu'à `n20q1000*.tsp`
- Chaque niveau de capacité a 10 variantes (A-J)

#### n30mos*, n30q* (30 nœuds)
- Mêmes familles mais avec 30 nœuds au lieu de 20

### Exemples de tests par famille
```bash
# Toutes les instances n20mos
Get-ChildItem "../Datasets/TS2004t2/n20mos*.tsp" | ForEach-Object {
    cargo run --release -- solve -i $_.FullName -a hybrid
}

# Instances avec capacité 10 (n20q10)
Get-ChildItem "../Datasets/TS2004t2/n20q10*.tsp" | ForEach-Object {
    cargo run --release -- solve -i $_.FullName -a greedy
}

# Instances 30 nœuds
Get-ChildItem "../Datasets/TS2004t2/n30*.tsp" | ForEach-Object {
    cargo run --release -- solve -i $_.FullName -a vnd -t 60
}
```

## Solveur exact (Gurobi)

### Configuration
1. Installer Gurobi 12.0+
2. Obtenir une licence (académique gratuite)
3. Configurer les variables d'environnement :
   ```powershell
   $env:GUROBI_HOME = "C:\gurobi1200\win64"
   $env:GRB_LICENSE_FILE = "C:\Users\<user>\gurobi.lic"
   ```
4. Recompiler avec Gurobi :
   ```bash
   cargo build --release --features gurobi
   ```

### Utilisation
```bash
cargo run --release --features gurobi -- solve -i ../Datasets/TS2004t2/n20mosA.tsp -a exact -t 300 -v
```

**Note** : Le solveur exact utilise une formulation MIP et peut être très lent sur les grandes instances.

## Développement

### Structure du projet
```
pd-tsp-solver/
├── src/
│   ├── main.rs              # CLI principale
│   ├── instance.rs          # Représentation de l'instance
│   ├── solution.rs          # Représentation de la solution
│   ├── heuristics/
│   │   ├── construction.rs  # Heuristiques constructives
│   │   ├── local_search.rs  # Recherches locales
│   │   ├── genetic.rs       # Algorithmes génétiques
│   │   ├── aco.rs          # Algorithmes de fourmis
│   │   └── profit_density.rs  # Notre heuristique custom
│   ├── exact/
│   │   └── gurobi.rs       # Solveur exact
│   ├── benchmark.rs         # Framework de benchmarking
│   └── visualization.rs     # Génération de SVG
├── Cargo.toml
└── README.md
```

### Ajouter un nouvel algorithme
1. Implémenter le trait `ConstructionHeuristic` ou `LocalSearchOperator`
2. Ajouter l'algorithme dans `main.rs` (enum `Algorithm` et match statement)
3. Tester sur les instances de référence

## Résultats attendus

### Heuristiques constructives (n20mosA)
| Algorithme | Coût | Faisable | Temps |
|------------|------|----------|-------|
| Greedy | 4656 | ✅ | < 1ms |
| NN | 5038 | ✅ | < 1ms |
| Regret | 4976 | ✅ | < 1ms |
| Sweep | 5882 | ✅ | < 1ms |
| Multi-start | ~4600 | ✅ | < 10ms |

### Métaheuristiques (n20mosA, 30s)
| Algorithme | Coût typique | Faisable |
|------------|--------------|----------|
| VND | ~4400 | ✅ |
| SA | ~4300 | ✅ |
| Hybrid | ~4200 | ✅ |

**Note** : Les résultats varient selon la graine aléatoire et le temps d'exécution.

## Bugs connus et limitations

1. **Instances difficiles** : Certaines instances (ex: n20mosD) ne permettent pas de solutions faisables avec les heuristiques simples
2. **Cluster-First** : Peut générer des solutions infaisables sur certaines instances
3. **Gurobi** : Nécessite une licence et une configuration manuelle

## Licence

MIT

## Auteurs

Projet M2 AI2D - MAOA 2026
