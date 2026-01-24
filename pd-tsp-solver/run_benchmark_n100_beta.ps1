# Script de benchmark optimisé pour le rapport
# Teste les algorithmes représentatifs sur les instances n100q45 et n100q1000

$ErrorActionPreference = "Continue"

# Créer le répertoire de résultats
$resultDir = "results\benchmark_complete"
New-Item -ItemType Directory -Force -Path $resultDir | Out-Null

# Instances à tester (sélection représentative)
$instances = @(
    "benchmark_n100\n100q45\n100q45A.tsp",
    "benchmark_n100\n100q45\n100q45B.tsp",
    "benchmark_n100\n100q45\n100q45C.tsp",
    "benchmark_n100\n100q1000\n100q1000A.tsp",
    "benchmark_n100\n100q1000\n100q1000B.tsp",
    "benchmark_n100\n100q1000\n100q1000C.tsp"
)

# Algorithmes représentatifs de chaque catégorie
# Heuristiques gloutonnes: 3 meilleurs
$constructive_algos = @("greedy", "regret", "profit-density")

# Méthodes itératives: les plus efficaces
$iterative_algos = @("vnd", "sa", "ils", "hybrid")

# Méthode exacte
$exact_algo = "exact"

# Tous les algorithmes
$all_algos = $constructive_algos + $iterative_algos + @($exact_algo)

# Fichier CSV pour les résultats
$csvFile = "$resultDir\results_beta.csv"

# Initialiser le CSV (alpha, beta fields appended)
"instance,algorithm,cost_function,cost,feasible,time,alpha,beta" | Out-File -FilePath $csvFile -Encoding UTF8

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "BENCHMARK OPTIMISÉ - PD-TSP SOLVER" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "Instances: $($instances.Count)"
Write-Host "Algorithmes: $($all_algos.Count)"
Write-Host "Temps limite: 100s par algorithme"
Write-Host "============================================`n" -ForegroundColor Cyan

$totalTests = 0
foreach ($instance in $instances) {
    foreach ($algo in $all_algos) {
        $totalTests += 2  # linéaire + quadratique (sauf exact)
        if ($algo -eq "exact") { $totalTests -= 1 }
    }
}

$currentTest = 0

foreach ($instance in $instances) {
    $instName = Split-Path $instance -Leaf
    Write-Host "`n========== Instance: $instName ==========" -ForegroundColor Yellow
    
    foreach ($algo in $all_algos) {
        # ===== TEST 1: Coût linéaire (distance) =====
        $currentTest++
        $progress = [math]::Round(($currentTest / $totalTests) * 100, 1)
        Write-Host "  [$progress%] $algo (linear)... " -NoNewline
        
        try {
            $output = & .\target\release\pd-tsp-solver.exe solve `
                --instance $instance `
                --algorithm $algo `
                --cost-function linear-load `
                --alpha 0.1 `
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
            
            "$instName,$algo,linear-load,$cost,$feasible,$time,0.1,0.0" | Out-File -FilePath $csvFile -Append -Encoding UTF8
            
            Write-Host "Cost=$cost, Feasible=$feasible, Time=$time" -ForegroundColor Green
        }
        catch {
            Write-Host "ERROR" -ForegroundColor Red
            "$instName,$algo,linear-load,ERROR,false,0,0.0" | Out-File -FilePath $csvFile -Append -Encoding UTF8
        }
        
        # ===== TEST 2: Coût quadratique (sauf exact) =====
        if ($algo -ne "exact") {
            # Test multiple beta values for the quadratic cost. We set alpha=0 and vary beta
            $beta_values = @(0.0, 0.001, 0.01, 0.1, 1.0)
            foreach ($beta in $beta_values) {
                $currentTest++
                $progress = [math]::Round(($currentTest / $totalTests) * 100, 1)
                Write-Host "  [$progress%] $algo (quadratic β=$beta)... " -NoNewline

                try {
                    # Use alpha=1 (default) and vary beta for quadratic sensitivity
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

                    "$instName,$algo,quadratic,$cost,$feasible,$time,1.0,$beta" | Out-File -FilePath $csvFile -Append -Encoding UTF8

                    Write-Host "Cost=$cost, Feasible=$feasible, Time=$time" -ForegroundColor Green
                }
                catch {
                    Write-Host "ERROR" -ForegroundColor Red
                    "$instName,$algo,quadratic,ERROR,false,0,1.0,$beta" | Out-File -FilePath $csvFile -Append -Encoding UTF8
                }
            }
        }
    }
}

Write-Host "`n============================================" -ForegroundColor Cyan
Write-Host "BENCHMARK TERMINÉ" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "Résultats sauvegardés: $csvFile"
Write-Host "`nUtilisez analyze_results.py pour analyser." -ForegroundColor Yellow
