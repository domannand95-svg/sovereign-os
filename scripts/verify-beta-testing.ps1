[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$baseline = "d3ce781fa647a24442b051b8b0e3a461881f0376"
$expectedAdapterBlob = "5fcb0216a13c2da1d2f78d71e66d0e95f438a303"
$fixtureRoot = "docs/experiments/local-agent-beta/fixtures/raw-output-adapter"

$expectedTxtHashes = [ordered]@{
    "$fixtureRoot/001_clean_exact_match.txt" = "b07b9b0b7679571b5b5dcda52e28ecd768b622d92ccf8e821fbe0a8976a26462"
    "$fixtureRoot/002_markdown_fenced_json.txt" = "bbdc8f42362cc1c9ba9d2ea1e1857660cc074a59fd0b48affb5e85d437ed529c"
    "$fixtureRoot/003_missing_required_fields.txt" = "4e14ec2b94e8a0e5a20363f04a49c06d2af407d1268dd4d7a55a2eb37dc236bf"
    "$fixtureRoot/004_hallucinated_schema_properties.txt" = "8b398a1f4e32b0988b79232b2d4ff016f35c78865d06b650d68250d425a1e2e4"
    "$fixtureRoot/005_trailing_garbage_text.txt" = "443327d355481704af1a894dc10b675d12f969c9f2421d13ccee269d6fbb550c"
    "$fixtureRoot/006_adversarial_context_request.txt" = "191949bac745ee9a49b37b49019e3510530a19cd0de5c6b87bd31066580ea47e"
    "$fixtureRoot/007_valid_context_request.txt" = "79ee0fbe9f5640c7287705a38319b0cbc9db4205e1ac985178a43f8e50dbbdce"
}

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)]
        [string]$FilePath,

        [Parameter(Mandatory = $true)]
        [string[]]$ArgumentList
    )

    & $FilePath @ArgumentList

    if ($LASTEXITCODE -ne 0) {
        throw "$FilePath failed with exit code $LASTEXITCODE"
    }
}

Push-Location $repo

try {
    Write-Host "=== Sovereign OS controlled beta-testing gate ==="

    Invoke-Checked git @("merge-base", "--is-ancestor", $baseline, "HEAD")

    $status = @(git status --porcelain=v1)
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to inspect Git status"
    }
    if ($status.Count -ne 0) {
        throw "Beta verification requires a clean Git worktree and index"
    }

    $adapterPath = "crates/beta001-harness/src/raw_output_adapter.rs"
    $adapterBlob = (git rev-parse "HEAD:$adapterPath").Trim()
    if ($LASTEXITCODE -ne 0 -or $adapterBlob -ne $expectedAdapterBlob) {
        throw "Frozen adapter Git object mismatch"
    }

    foreach ($relativePath in $expectedTxtHashes.Keys) {
        $absolutePath = Join-Path $repo $relativePath
        $actualHash = (Get-FileHash -LiteralPath $absolutePath -Algorithm SHA256).Hash.ToLowerInvariant()

        if ($actualHash -ne $expectedTxtHashes[$relativePath]) {
            throw "Frozen fixture SHA-256 mismatch: $relativePath"
        }

        $attributes = @(git check-attr text eol -- $relativePath)
        if ($LASTEXITCODE -ne 0) {
            throw "Unable to inspect Git attributes: $relativePath"
        }

        if (
            $attributes.Count -ne 2 -or
            $attributes[0] -ne "$relativePath`: text: set" -or
            $attributes[1] -ne "$relativePath`: eol: lf"
        ) {
            throw "Frozen fixture checkout policy mismatch: $relativePath"
        }
    }

    Invoke-Checked git @("diff", "--check")
    Invoke-Checked cargo @("fmt", "--all", "--", "--check")
    Invoke-Checked cargo @("clippy", "--workspace", "--all-targets", "--locked", "--offline", "--", "-D", "warnings")
    Invoke-Checked cargo @("test", "-p", "beta001-harness", "--test", "exp_beta_002_raw_output_adapter", "--locked", "--offline")
    Invoke-Checked cargo @("test", "-p", "beta001-harness", "--test", "exp_beta_002_rejection_taxonomy", "--locked", "--offline")
    Invoke-Checked cargo @("test", "-p", "beta001-harness", "--test", "exp_beta_002_normalization_semantic_boundary", "--locked", "--offline")
    Invoke-Checked cargo @("test", "--workspace", "--all-targets", "--locked", "--offline")

    $finalStatus = @(git status --porcelain=v1)
    if ($LASTEXITCODE -ne 0 -or $finalStatus.Count -ne 0) {
        throw "Repository state changed during beta verification"
    }

    Write-Host "BETA-TESTING GATE: PASS"
    Write-Host "Authority boundary: controlled non-production fixture evaluation only"
}
finally {
    Pop-Location
}
