param(
    [string]$ProjectPath = "C:\Users\idene\BlazeTest",
    [string]$UnityExe
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-UnityExeFromProject {
    param([string]$ResolvedProjectPath)

    $versionFile = Join-Path $ResolvedProjectPath "ProjectSettings\ProjectVersion.txt"
    if (-not (Test-Path -LiteralPath $versionFile)) {
        throw "Unity project version file not found: $versionFile"
    }

    $versionLine = Select-String -Path $versionFile -Pattern '^m_EditorVersion:\s*(.+)$' | Select-Object -First 1
    if (-not $versionLine) {
        throw "Unable to read Unity version from $versionFile"
    }

    $version = $versionLine.Matches[0].Groups[1].Value.Trim()
    return Join-Path $env:ProgramFiles "Unity\Hub\Editor\$version\Editor\Unity.exe"
}

function Get-OpenUnityProjectProcesses {
    param([string]$ResolvedProjectPath)

    $normalizedProjectPath = [IO.Path]::GetFullPath($ResolvedProjectPath)
    $unityProcesses = Get-CimInstance Win32_Process -Filter "Name = 'Unity.exe'"

    foreach ($process in $unityProcesses) {
        $commandLine = [string]$process.CommandLine
        if ([string]::IsNullOrWhiteSpace($commandLine)) {
            continue
        }

        $normalizedCommandLine = $commandLine.Replace('/', '\').ToLowerInvariant()
        if ($normalizedCommandLine.Contains($normalizedProjectPath.ToLowerInvariant()) -and -not $normalizedCommandLine.Contains('-batchmode')) {
            [pscustomobject]@{
                ProcessId = $process.ProcessId
                CommandLine = $commandLine
            }
        }
    }
}

$resolvedProjectPath = (Resolve-Path -LiteralPath $ProjectPath).Path
if (-not $UnityExe) {
    $UnityExe = Get-UnityExeFromProject -ResolvedProjectPath $resolvedProjectPath
}

$openUnityProcesses = @(Get-OpenUnityProjectProcesses -ResolvedProjectPath $resolvedProjectPath)
if ($openUnityProcesses.Count -gt 0) {
    $processList = ($openUnityProcesses | ForEach-Object { $_.ProcessId }) -join ', '
    throw "BlazeTest project is already open in Unity (PID: $processList). Close the editor and rerun the smoke script."
}

if (-not (Test-Path -LiteralPath $UnityExe)) {
    throw "Unity executable not found: $UnityExe"
}

$outputDir = Join-Path $PSScriptRoot "build-output\blazetest-smoke"
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

$resultsPath = Join-Path $outputDir "results.xml"
$logPath = Join-Path $outputDir "unity.log"

foreach ($artifactPath in @($resultsPath, $logPath)) {
    if (Test-Path -LiteralPath $artifactPath) {
        Remove-Item -LiteralPath $artifactPath -Force
    }
}

$arguments = @(
    "-batchmode",
    "-nographics",
    "-projectPath", $resolvedProjectPath,
    "-runTests",
    "-testPlatform", "EditMode",
    "-testResults", $resultsPath,
    "-logFile", $logPath
)

Write-Host "Unity executable: $UnityExe"
Write-Host "Project path: $resolvedProjectPath"
Write-Host "Results: $resultsPath"
Write-Host "Log: $logPath"

$process = Start-Process -FilePath $UnityExe -ArgumentList $arguments -NoNewWindow -Wait -PassThru
$exitCode = [int]$process.ExitCode

if (Test-Path -LiteralPath $resultsPath) {
    Write-Host "Unity test results written to $resultsPath"
}
else {
    throw "BlazeTest smoke run finished without writing test results. See $logPath"
}

if ($exitCode -ne 0) {
    throw "BlazeTest smoke run failed with exit code $exitCode. See $logPath"
}

Write-Host "BlazeTest smoke run completed successfully."
