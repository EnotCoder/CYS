param([Parameter(ValueFromRemainingArguments = $true)] [string[]]$Rest)

# Backward-compatible entry point; the script lives in build/windows/.
& "$PSScriptRoot\..\build\windows\build.ps1" @Rest
exit $LASTEXITCODE
