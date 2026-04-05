param(
    [switch]$SkipRust,
    [switch]$SkipVsCode,
    [switch]$SkipUnity,
    [string]$ProjectPath = "C:\Users\idene\BlazeTest",
    [string]$UnityExe
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Invoke-NativeStep {
    param(
        [string]$Name,
        [string]$WorkingDirectory,
        [string]$FilePath,
        [string[]]$Arguments
    )

    Write-Host ""
    Write-Host "== $Name =="
    Write-Host "cwd: $WorkingDirectory"
    Write-Host "cmd: $FilePath $($Arguments -join ' ')"

    Push-Location $WorkingDirectory
    try {
        & $FilePath @Arguments
        $exitCode = if ($null -ne $LASTEXITCODE) { [int]$LASTEXITCODE } else { 0 }
        if ($exitCode -ne 0) {
            throw "$Name failed with exit code $exitCode."
        }
    }
    finally {
        Pop-Location
    }
}

function Invoke-PowershellScriptStep {
    param(
        [string]$Name,
        [string]$ScriptPath,
        [hashtable]$Parameters
    )

    Write-Host ""
    Write-Host "== $Name =="
    Write-Host "script: $ScriptPath"

    & $ScriptPath @Parameters
    $exitCode = if ($null -ne $LASTEXITCODE) { [int]$LASTEXITCODE } else { 0 }
    if ($exitCode -ne 0) {
        throw "$Name failed with exit code $exitCode."
    }
}

$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$startTime = Get-Date

if (-not $SkipRust) {
    Invoke-NativeStep `
        -Name "Rust tests" `
        -WorkingDirectory $repoRoot `
        -FilePath "cargo" `
        -Arguments @("test")
}

if (-not $SkipVsCode) {
    Invoke-NativeStep `
        -Name "VS Code extension tests" `
        -WorkingDirectory (Join-Path $repoRoot "vscode-prsm") `
        -FilePath "npm" `
        -Arguments @("test")
}

if (-not $SkipUnity) {
    $unityParameters = @{
        ProjectPath = $ProjectPath
    }

    if ($UnityExe) {
        $unityParameters["UnityExe"] = $UnityExe
    }

    Invoke-PowershellScriptStep `
        -Name "BlazeTest Unity smoke" `
        -ScriptPath (Join-Path $repoRoot "run-blazetest-smoke.ps1") `
        -Parameters $unityParameters
}

$duration = (Get-Date) - $startTime
Write-Host ""
Write-Host ("Verification completed successfully in {0:mm\:ss}." -f $duration)
