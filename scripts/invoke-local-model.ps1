[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Task,

    [Parameter(Mandatory = $true)]
    [string]$ContextFile,

    [Parameter(Mandatory = $true)]
    [string]$OutputFile,

    [string]$Model = "deepseek-coder:6.7b",

    [string]$Uri = "http://localhost:11434/api/generate"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $ContextFile)) {
    throw "Context file not found at path: $ContextFile"
}

Write-Host "[-] Loading workspace context from: $ContextFile" -ForegroundColor Cyan
$ContextContent = Get-Content -Path $ContextFile -Raw

$Prompt = @"
You are an expert Rust systems programmer contributing to sovereign-os, a zero-trust, cryptographically-verified operating system.

WORKSPACE CONTEXT:
$ContextContent

TASK:
$Task

CONSTRAINTS & ENGINEERING STANDARDS:
- Zero-tolerance linting: cargo clippy -- -D warnings
- No blanket #[allow(...)] in production code
- All errors must be explicit, typed, and auditable
- Stateless translators, fail-fast validation logic
- Absolutely no unwrap() or expect() in production code paths
- Follow existing workspace crate structures and idioms strictly

Generate the complete, compilation-ready code block with all necessary imports included. Do not truncate.
"@

Write-Host "[-] Dispatching generation request to local model ($Model)..." -ForegroundColor Cyan

$Body = @{
    model  = $Model
    prompt = $Prompt
    stream = $false
    options = @{
        temperature = 0.2
        num_predict = 4096
    }
} | ConvertTo-Json -Depth 5

try {
    $Response = Invoke-RestMethod -Uri $Uri -Method Post -Body $Body -ContentType "application/json"
    
    if ($Response.response) {
        $DestinationDir = Split-Path -Parent $OutputFile
        if ($DestinationDir -and -not (Test-Path $DestinationDir)) {
            New-Item -ItemType Directory -Force -Path $DestinationDir | Out-Null
        }

        $CleanedCode = $Response.response
        if ($CleanedCode -match '(?s)```rust\s*(.*?)\s*```') {
            $CleanedCode = $Matches[1]
        }

        $CleanedCode | Out-File -FilePath $OutputFile -Encoding UTF8
        Write-Host "[+] Successfully generated and saved: $OutputFile" -ForegroundColor Green
    } else {
        throw "Received empty response payload from local model API."
    }
}
catch {
    Write-Error "Failed to invoke local model API: $_"
    exit 1
}
