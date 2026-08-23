[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Task,

    [Parameter(Mandatory = $true)]
    [string]$ContextFile,

    [Parameter(Mandatory = $true)]
    [string]$OutputFile,

    [string]$TestCommand = "cargo test --test runtime_hook_tests",

    [int]$MaxRetries = 3
)

$ErrorActionPreference = "Stop"
$Attempt = 1
$Success = $false

while ($Attempt -le $MaxRetries) {
    Write-Host "`n[+] Autonomous Loop — Iteration $Attempt of $MaxRetries" -ForegroundColor Yellow

    # Step 1: Run local model harness generation
    Write-Host "[-] Invoking local model generation..." -ForegroundColor Cyan
    ./scripts/invoke-local-model.ps1 -Task $Task -Context $ContextFile -OutputFile $OutputFile

    # Step 2: Run verification command
    Write-Host "[-] Running validation suite: $TestCommand" -ForegroundColor Cyan
    
     = \Stop
\Stop = "SilentlyContinue"
\ = Invoke-Expression "\ 2>&1"
\ = \-1
\Stop = \

    if ($ExitCode -eq 0) {
        Write-Host "[+] Build and validation passed successfully on iteration $Attempt!" -ForegroundColor Green
        $Success = $true
        break
    } else {
        Write-Warning "[-] Validation failed with exit code $ExitCode. Capturing compiler errors for self-correction..."
        
        # Grab the tail of the error output to feed back as context
        $ErrorSnippet = ($OutputCapture | Select-Object -Last 15) -join "`n"
        
        # Dynamically update the task prompt with real-time error feedback for the next loop
        $Task = "Fix the previous implementation in $OutputFile. Address these exact compiler errors:`n$ErrorSnippet`nEnsure strict clippy compliance, explicit error handling, and zero unwrap()."
    }

    $Attempt++
}

if (-not $Success) {
    throw "[-] Autonomous agent failed to achieve a clean compile after $MaxRetries attempts."
}

