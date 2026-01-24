# Quick benchmark for n200 instances
$ErrorActionPreference = "Continue"

# Create results directory
$resultDir = "results\benchmark_n200_quick"
New-Item -ItemType Directory -Force -Path $resultDir | Out-Null

# Gather all .tsp instances from benchmark_n200
$instances = Get-ChildItem -Path "benchmark_n200" -Filter "*.tsp" | ForEach-Object { Join-Path "benchmark_n200" $_.Name }

# Representative algorithms (quick)
$algos = @("greedy", "regret", "vnd", "ils", "hybrid")

$csvFile = "$resultDir\results_n200_quick.csv"
"instance,algorithm,cost_function,cost,feasible,time,alpha,beta" | Out-File -FilePath $csvFile -Encoding UTF8

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "BENCHMARK N200 QUICK - PD-TSP SOLVER" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "Instances: $($instances.Count)"
Write-Host "Algorithmes: $($algos.Count)"
Write-Host "Temps limite: 30s par algorithme" -ForegroundColor Cyan

foreach ($instance in $instances) {
    $instName = Split-Path $instance -Leaf
    Write-Host "\n===== Instance: $instName =====" -ForegroundColor Yellow
    foreach ($algo in $algos) {
        Write-Host "  Testing $algo (linear-load)... " -NoNewline
        try {
            $output = & .\target\release\pd-tsp-solver.exe solve `
                --instance $instance `
                --algorithm $algo `
                --cost-function linear-load `
                --alpha 0.1 `
                --time-limit 60 `
                --seed 42 2>&1 | Out-String

            if ($output -match "Cost \(travel\):\s+([\d.]+)") { $cost = $matches[1] } elseif ($output -match "Cost:\s+([\d.]+)") { $cost = $matches[1] } else { $cost = "N/A" }
            if ($output -match "Feasible:\s+(\w+)") { $feasible = $matches[1] } else { $feasible = "unknown" }
            if ($output -match "Time:\s+([\d.]+)s") { $time = $matches[1] } else { $time = "N/A" }

            "$instName,$algo,linear-load,$cost,$feasible,$time,0.1,0.0" | Out-File -FilePath $csvFile -Append -Encoding UTF8
            Write-Host "Cost=$cost, Feasible=$feasible, Time=$time" -ForegroundColor Green
        } catch {
            Write-Host "ERROR" -ForegroundColor Red
            "$instName,$algo,distance,ERROR,false,0,0.0" | Out-File -FilePath $csvFile -Append -Encoding UTF8
        }
    }
}

Write-Host "\nBenchmark n200 quick finished. Results: $csvFile" -ForegroundColor Cyan
