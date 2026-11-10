# Install the Fragment Rust CLI.
#
#   irm https://raw.githubusercontent.com/Jesse-Dan/fragment-rs/main/scripts/install.ps1 | iex
#
# Set FRS_REPO to install from a different repository, and FRS_INSTALL_DIR to
# install somewhere other than ~/.local/bin.
$ErrorActionPreference = "Stop"

$repo = if ($env:FRS_REPO) { $env:FRS_REPO } else { "Jesse-Dan/fragment-rs" }
$dest = if ($env:FRS_INSTALL_DIR) { $env:FRS_INSTALL_DIR } else { Join-Path $HOME ".local\bin" }
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())

New-Item -ItemType Directory -Path $tmp | Out-Null
try {
    Invoke-WebRequest -Uri "https://github.com/$repo/releases/latest/download/frs-windows-x64.zip" -OutFile "$tmp\frs.zip"
    Expand-Archive -Path "$tmp\frs.zip" -DestinationPath $tmp -Force

    New-Item -ItemType Directory -Force -Path $dest | Out-Null
    Copy-Item "$tmp\frs.exe" "$dest\frs.exe" -Force

    # Put the directory on the PATH so `frs` runs from anywhere.
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$dest*") {
        [Environment]::SetEnvironmentVariable("Path", "$userPath;$dest", "User")
        Write-Host "Added $dest to your PATH. Open a new terminal."
    }

    Write-Host "Installed frs to $dest\frs.exe"
    & "$dest\frs.exe" --version
}
finally {
    Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
