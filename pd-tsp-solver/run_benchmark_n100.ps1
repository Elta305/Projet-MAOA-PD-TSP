# Script de benchmark complet pour le rapport
# Teste tous les algorithmes sur les instances n100q45 et n100q1000
# Temps limite: 100 secondes par algorithme par instance

$ErrorActionPreference = "Continue"

# Créer le répertoire de résultats
$resultDir = "results\benchmark_complete_n100"
New-Item -ItemType Directory -Force -Path $resultDir | Out-Null

# Instances à tester
$instances = @(
    "benchmark_n100\n100q45\n100q45A.tsp",
    "benchmark_n100\n100q45\n100q45B.tsp",
    "benchmark_n100\n100q45\n100q45C.tsp",
    "benchmark_n100\n100q1000\n100q1000A.tsp",
    "benchmark_n100\n100q1000\n100q1000B.tsp",
    "benchmark_n100\n100q1000\n100q1000C.tsp"
)

# Algorithmes à tester
# Heuristiques gloutonnes
$constructive_algos = @("nn", "greedy", "savings", "sweep", "regret", "cluster-first", "profit-density")

# Méthodes itératives
$iterative_algos = @("two-opt", "vnd", "sa", "tabu", "ils", "ga", "memetic", "aco", "mmas", "hybrid")

# Méthode exacte (uniquement sur coût linéaire)
$exact_algo = "exact"

# Tous les algorithmes
$all_algos = $constructive_algos + $iterative_algos + @($exact_algo)

# Fichier CSV pour les résultats
$csvFile = "$resultDir\results.csv"
$csvFileQuad = "$resultDir\results_quadratic.csv"

# Initialiser les CSV
"instance,algorithm,cost_function,cost,feasible,time,alpha,beta" | Out-File -FilePath $csvFile -Encoding UTF8
"instance,algorithm,cost_function,cost,feasible,time,alpha,beta" | Out-File -FilePath $csvFileQuad -Encoding UTF8

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "BENCHMARK COMPLET - PD-TSP SOLVER" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "Instances: $($instances.Count)"
Write-Host "Algorithmes: $($all_algos.Count)"
Write-Host "Temps limite: 100s par algorithme"
Write-Host "============================================`n" -ForegroundColor Cyan

$totalTests = $instances.Count * $all_algos.Count * 2  # x2 pour linéaire et quadratique
$currentTest = 0

foreach ($instance in $instances) {
    $instName = Split-Path $instance -Leaf
    Write-Host "`n========== Instance: $instName ==========" -ForegroundColor Yellow
    
    foreach ($algo in $all_algos) {
        # ===== TEST 1: Coût linéaire (distance) =====
        $currentTest++
        $progress = [math]::Round(($currentTest / $totalTests) * 100, 1)
        Write-Host "  [$progress%] Testing $algo (linear)... " -NoNewline
        
            try {
            # Exécuter l'algorithme avec fonction de coût linéaire
            $output = & .\target\release\pd-tsp-solver.exe solve `
                --instance $instance `
                --algorithm $algo `
                    --cost-function linear-load `
                    --alpha 0.1 `
                --time-limit 100 `
                --seed 42 2>&1 | Out-String
            
            # Parser les résultats
            if ($output -match "Cost \(travel\):\s+([\d.]+)") {
                $cost = $matches[1]
            } elseif ($output -match "Cost:\s+([\d.]+)") {
                $cost = $matches[1]
            } else {
                $cost = "N/A"
            }
            
            if ($output -match "Feasible:\s+(\w+)") {
                $feasible = $matches[1]
            } else {
                $feasible = "unknown"
            }
            
            if ($output -match "Time:\s+([\d.]+)s") {
                $time = $matches[1]
            } else {
                $time = "N/A"
            }
            
            # Écrire dans le CSV
            "$instName,$algo,linear-load,$cost,$feasible,$time,0.1,0.0" | Out-File -FilePath $csvFile -Append -Encoding UTF8
            
            Write-Host "Cost=$cost, Time=$time" -ForegroundColor Green
        }
        catch {
            Write-Host "ERROR" -ForegroundColor Red
            "$instName,$algo,linear-load,ERROR,false,0,0.1,0.0" | Out-File -FilePath $csvFile -Append -Encoding UTF8
        }
        
        # ===== TEST 2: Coût quadratique (sauf exact) =====
        if ($algo -ne "exact") {
            $currentTest++
            $progress = [math]::Round(($currentTest / $totalTests) * 100, 1)
            Write-Host "  [$progress%] Testing $algo (quadratic)... " -NoNewline
            
            try {
                # Tester avec une valeur de beta (alpha=0)
                $beta = 0.01

                    $output = & .\target\release\pd-tsp-solver.exe solve `
                    --instance $instance `
                    --algorithm $algo `
                    --cost-function quadratic `
                    --alpha 1 `
                    --beta $beta `
                    --time-limit 100 `
                    --seed 42 2>&1 | Out-String
                
                if ($output -match "Cost \(travel\):\s+([\d.]+)") {
                    $cost = $matches[1]
                } elseif ($output -match "Cost:\s+([\d.]+)") {
                    $cost = $matches[1]
                } else {
                    $cost = "N/A"
                }
                
                if ($output -match "Feasible:\s+(\w+)") {
                    $feasible = $matches[1]
                } else {
                    $feasible = "unknown"
                }
                
                if ($output -match "Time:\s+([\d.]+)s") {
                    $time = $matches[1]
                } else {
                    $time = "N/A"
                }
                
                "$instName,$algo,quadratic,$cost,$feasible,$time,1.0,$beta" | Out-File -FilePath $csvFileQuad -Append -Encoding UTF8
                
                Write-Host "Cost=$cost, Time=$time" -ForegroundColor Green
            }
            catch {
                Write-Host "ERROR" -ForegroundColor Red
                "$instName,$algo,quadratic,ERROR,false,0,1.0,$beta" | Out-File -FilePath $csvFileQuad -Append -Encoding UTF8
            }
        } else {
            # Pour exact, on saute le test quadratique
            $currentTest++
            Write-Host "  [SKIP] exact (quadratic not supported)" -ForegroundColor DarkGray
        }
    }
}

Write-Host "`n============================================" -ForegroundColor Cyan
Write-Host "BENCHMARK TERMINÉ" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "Résultats sauvegardés dans:"
Write-Host "  - $csvFile"
Write-Host "  - $csvFileQuad"
Write-Host "`nUtilisez analyze_results.py pour analyser les données." -ForegroundColor Yellow
