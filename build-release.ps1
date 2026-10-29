param(
    [string]$Target = "x86_64-pc-windows-msvc"
)

$ErrorActionPreference = "Stop"
$projectRoot = $PSScriptRoot
$targetInfo = & rustc -vV
if ($LASTEXITCODE -ne 0) {
    throw "rustc is unavailable. Install the Rust toolchain first."
}
$hostLine = $targetInfo | Where-Object { $_ -like "host:*" } | Select-Object -First 1
$hostTarget = ($hostLine -split ":", 2)[1].Trim()
if ($Target -notin @("x86_64-pc-windows-msvc", "i686-pc-windows-msvc")) {
    throw "This packaging script currently supports Windows MSVC targets only."
}

Push-Location $projectRoot
try {
    & cargo build --locked --release --lib --target $Target
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo build failed for $Target."
    }

    $targetRelease = Join-Path $projectRoot "target\$Target\release"
    $dllPath = Join-Path $targetRelease "truespeech.dll"
    if (-not (Test-Path -LiteralPath $dllPath -PathType Leaf)) {
        throw "Expected DLL was not produced: $dllPath"
    }
    $importLibrary = Join-Path $targetRelease "truespeech.dll.lib"
    if (-not (Test-Path -LiteralPath $importLibrary -PathType Leaf)) {
        throw "Expected MSVC import library was not produced: $importLibrary"
    }

    $architecture = if ($Target.StartsWith("x86_64-")) { "windows-x64" } else { "windows-x86" }
    $distribution = Join-Path $projectRoot "dist\$architecture"
    New-Item -ItemType Directory -Force -Path $distribution | Out-Null
    Copy-Item -LiteralPath $dllPath -Destination $distribution -Force
    Copy-Item -LiteralPath $importLibrary -Destination $distribution -Force
    Copy-Item -LiteralPath (Join-Path $projectRoot "include\truespeech.h") -Destination $distribution -Force
    Copy-Item -LiteralPath (Join-Path $projectRoot "README.md") -Destination $distribution -Force
    Copy-Item -LiteralPath (Join-Path $projectRoot "examples\use_dll.c") -Destination $distribution -Force
    Copy-Item -LiteralPath (Join-Path $projectRoot "docs\api.md") -Destination $distribution -Force
    $documentation = Join-Path $distribution "docs"
    New-Item -ItemType Directory -Force -Path $documentation | Out-Null
    Copy-Item -Path (Join-Path $projectRoot "docs\*") -Destination $documentation -Recurse -Force
    foreach ($projectDocument in @("CONTRIBUTING.md", "CODE_OF_CONDUCT.md", "SECURITY.md")) {
        Copy-Item -LiteralPath (Join-Path $projectRoot $projectDocument) -Destination $distribution -Force
    }
    Copy-Item -LiteralPath (Join-Path $projectRoot "LICENSE") -Destination $distribution -Force

    $rustVersion = (& rustc --version).Trim()
    $packageVersion = if ((Get-Content -LiteralPath (Join-Path $projectRoot "Cargo.toml") -Raw) -match '(?m)^version\s*=\s*"([^"]+)"') {
        $Matches[1]
    } else {
        throw "Could not read the package version from Cargo.toml."
    }
    @(
        "Package: truespeech $packageVersion"
        "Target: $Target"
        "Rust: $rustVersion"
        "Host: $hostTarget"
        "Build: release"
        ""
        "Files: truespeech.dll, truespeech.dll.lib, truespeech.h, use_dll.c, README.md, docs/, CONTRIBUTING.md, CODE_OF_CONDUCT.md, SECURITY.md, LICENSE"
    ) | Set-Content -LiteralPath (Join-Path $distribution "BUILD-INFO.txt") -Encoding utf8

    Get-ChildItem -LiteralPath $distribution -File -Recurse |
        Where-Object { $_.FullName -ne (Join-Path $distribution "SHA256SUMS.txt") } |
        Sort-Object FullName |
        ForEach-Object {
            $hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            $relativePath = $_.FullName.Substring($distribution.Length).TrimStart([char[]]@("\", "/")).Replace("\", "/")
            "$hash  $relativePath"
        } | Set-Content -LiteralPath (Join-Path $distribution "SHA256SUMS.txt") -Encoding ascii

    Write-Output "Distribution ready: $distribution"
}
finally {
    Pop-Location
}
