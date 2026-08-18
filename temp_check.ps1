$files = Get-ChildItem -Path crates/oz-core/src/db -Recurse -Filter *.rs | Select-Object -ExpandProperty FullName
$remaining = @()
foreach ($file in $files) {
    $content = Get-Content -Path $file -Raw
    if ($content -match '#\[cfg\(test\)\]\s*mod\s+tests\s*\{') {
        $remaining += $file
    }
}
if ($remaining.Count -gt 0) {
    Write-Host 'FILES WITH REMAINING INLINE TEST BLOCKS:'
    $remaining | ForEach-Object { Write-Host $_ }
} else {
    Write-Host 'NO REMAINING INLINE TEST BLOCKS FOUND IN DB FILES.'
}
Write-Host "`nRunning cargo test -p oz-core (this may take a moment)..."
cargo test -p oz-core