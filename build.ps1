[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
$sources = @(
    (Join-Path $root 'src\Core.cs'),
    (Join-Path $root 'src\Ui.cs'),
    (Join-Path $root 'src\MainWindow.cs'),
    (Join-Path $root 'src\MainWindow.Actions.cs')
)
$dist = Join-Path $root 'dist'
$artifacts = Join-Path $root 'artifacts'
$compilerCandidates = @(
    "$env:WINDIR\Microsoft.NET\Framework64\v4.0.30319\csc.exe",
    "$env:WINDIR\Microsoft.NET\Framework\v4.0.30319\csc.exe"
)
$compiler = $compilerCandidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1

if (-not $compiler) {
    throw 'Windows .NET Framework compiler not found.'
}

New-Item -ItemType Directory -Force -Path $dist, $artifacts | Out-Null
$framework = Split-Path -Parent $compiler
$references = @(
    '/reference:System.dll',
    '/reference:System.Core.dll',
    "/reference:$framework\System.Xaml.dll",
    "/reference:$framework\WPF\WindowsBase.dll",
    "/reference:$framework\WPF\PresentationCore.dll",
    "/reference:$framework\WPF\PresentationFramework.dll"
)

& $compiler /nologo /optimize+ /target:exe /platform:anycpu /main:LocalPass.SelfTestProgram "/out:$artifacts\LocalPass.SelfTest.exe" @references @sources
if ($LASTEXITCODE -ne 0) { throw 'Self-test build failed.' }

& "$artifacts\LocalPass.SelfTest.exe"
if ($LASTEXITCODE -ne 0) { throw 'Self-test failed.' }

& $compiler /nologo /optimize+ /target:winexe /platform:anycpu /main:LocalPass.Program "/out:$dist\LocalPass.exe" @references @sources
if ($LASTEXITCODE -ne 0) { throw 'Application build failed.' }

$output = Get-Item -LiteralPath "$dist\LocalPass.exe"
Write-Host "Built $($output.FullName) ($([Math]::Round($output.Length / 1KB)) KB)"
