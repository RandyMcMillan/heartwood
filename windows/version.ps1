$prefix = "releases"

$describe = git describe --match "$prefix/*.*.*" --exclude "$prefix/*.*.*-*.*" --candidates=1 2>$null
if ($LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($describe) -and $describe.Trim() -match "^$prefix/(\d+)\.(\d+)\.(\d+)(?:-(\d+)-g[0-9a-f]+)?$") {
  $major = [int]$matches[1]
  $minor = [int]$matches[2]
  $patch = [int]$matches[3]
  $commits = if ([string]::IsNullOrEmpty($matches[4])) { 0 } else { [int]$matches[4] }
  Write-Output "$major.$minor.$patch.$commits"
  exit 0
}

$count = git rev-list --count HEAD 2>$null
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($count)) {
  $count = "0"
}

Write-Output "0.0.0.$([int]$count)"
