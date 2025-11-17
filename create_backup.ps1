# Backup script for Percolator workspace
# Excludes node_modules, target, .git, and other build artifacts

$sourceDir = "percolator"
$backupName = "percolator_backup_$(Get-Date -Format 'yyyyMMdd_HHmmss').zip"
$tempDir = "percolator_backup_temp"

# Clean up temp directory if it exists
if (Test-Path $tempDir) {
    Remove-Item -Path $tempDir -Recurse -Force
}

# Create temp directory
New-Item -ItemType Directory -Path $tempDir | Out-Null

# Copy files excluding unwanted directories
$excludeDirs = @('node_modules', 'target', '.git', 'dist', 'build', '.next', 'registry', 'git')
$excludePatterns = @('node_modules', 'target', '\.git', 'dist', 'build', '\.next', '\.cargo\\registry', '\.cargo\\git')

Get-ChildItem -Path $sourceDir -Recurse -File | Where-Object {
    $relativePath = $_.FullName.Substring((Resolve-Path $sourceDir).Path.Length + 1)
    $shouldExclude = $false
    foreach ($pattern in $excludePatterns) {
        if ($relativePath -match $pattern) {
            $shouldExclude = $true
            break
        }
    }
    -not $shouldExclude
} | ForEach-Object {
    $relativePath = $_.FullName.Substring((Resolve-Path $sourceDir).Path.Length + 1)
    $destPath = Join-Path $tempDir $relativePath
    $destDir = Split-Path $destPath -Parent
    if (-not (Test-Path $destDir)) {
        New-Item -ItemType Directory -Path $destDir -Force | Out-Null
    }
    Copy-Item -Path $_.FullName -Destination $destPath -Force
}

# Create zip archive
Write-Host "Creating backup archive: $backupName"
Compress-Archive -Path $tempDir\* -DestinationPath $backupName -CompressionLevel Optimal -Force

# Clean up temp directory
Remove-Item -Path $tempDir -Recurse -Force

Write-Host "Backup created successfully: $backupName"
Write-Host "Size: $([math]::Round((Get-Item $backupName).Length / 1MB, 2)) MB"

