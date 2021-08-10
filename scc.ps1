param(
    [string]
    [ValidateSet("check", "build", "run", "regen", "ovn-reviews")]
    [Parameter(
        Mandatory = $true,
        HelpMessage = "The command to be run"
    )]
    $Command,
    [switch] $Release,
    [switch] $OutputInternal,
    [switch] $Rebuild,
    [string] $DataFile,
    [switch] $Transformer,
    [switch] $NoDDShow,
    [byte] $Workers = 4
)

if (!$PSScriptRoot) {
    Write-Error "`$PSScriptRoot was not defined, not run from within a script file"
    Exit 1

} elseif ($Workers -Eq 0) {
    Write-Error "Workers must be a non-zero integer"
    Exit 1

} elseif (-Not (Get-Command -Name "ddlog" -ErrorAction Stop)) {
    Write-Error "Could not find ddlog executable on the current path"
    Exit 1

} elseif (-Not (Get-Command -Name "cargo" -ErrorAction Stop)) {
    Write-Error "Could not find cargo executable on the current path"
    Exit 1

} elseif ($Command -Eq "run" -And -Not (Get-Command -Name "ddshow" -ErrorAction Stop)) {
    Write-Error "Could not find ddshow executable on the current path"
    Exit 1

} elseif ($Command -Eq "regen" -And -Not (Get-Command -Name "python" -ErrorAction Stop)) {
    Write-Error "Could not find python executable on the current path"
    Exit 1

} elseif ($Command -Ne "run" -And $Rebuild) {
    Write-Error "Can only pass -Rebuild with the 'run' subcommand"
    Exit 1

} elseif ($Command -Ne "run" -And $NoDDShow) {
    Write-Error "Can only pass -NoDDShow with the 'run' subcommand"
    Exit 1
}

$ReleaseFlags = if ($Release) { "--release" } else { "" }
$OutFolder = if ($Release) { "release" } else { "debug" }
$Prefix = if ($Transformer) { "scc_transformer" } else { "scc" }
$DataFile = if ($DataFile) { $DataFile } else { "$PSScriptRoot/scc.dat" }
$CodeDir = "$PSScriptRoot\$Prefix"
$RustDir = "$CodeDir\$Prefix`_ddlog"
$TraceDir = "$CodeDir\traces"
$Executable = "$RustDir\target\$OutFolder\$Prefix`_cli.exe"
$DDlogFile = "$Prefix.dl"

function Regen-Input {
    Write-Host "Generating scc.dat..."

    # Run the python script to regenerate the graph data
    python "$PSScriptRoot/graph.py"

    if ($LastExitCode -ne 0) {
        Write-Error "failed to run the regeneration script, exiting"
        Exit $LastExitCode
    }
}

function Build-Executable {
    # Get the current rustflags and if they're not set or are empty
    # override them to use lld for linking and target the native cpu 
    $OldRustflags = $env:RUSTFLAGS;
    if (!$OldRustflags -Or $OldRustflags -Eq "") {
        $env:RUSTFLAGS = "-C link-arg=-fuse-ld=lld -C target-cpu=native"
    }

    try {
        # cd into the directory that contains the target `.dl` file
        Push-Location -Path $CodeDir
        $DDlogFlags = if ($OutputInternal) { "--output-internal-relations" } else { "" }

        # Run ddlog on the target file
        ddlog -i $DDlogFile --omit-profile --omit-workspace --no-staticlib $DDlogFlags | Write-Host
        $ExitCode = $LastExitCode
        Pop-Location

        # If ddlog failed, exit
        if ($ExitCode -ne 0) {
            return $ExitCode
        }

        # Rewrite all the dependencies on `ddlog-dev/ddshow` to point at `Kixiron/ddshow` since
        # the ddlog-dev repo is behind on ddshow releases
        $Files = Get-ChildItem $RustDir -Include "Cargo.toml" -Recurse | Where-Object { Test-Path $_.FullName -PathType Leaf }
        foreach ($File in $Files) {
            $Contents = Get-Content $File.FullName | Out-String

            if ($Contents -Match "ddshow-sink" ) {
                $Contents -Replace "`"https://github.com/ddlog-dev/ddshow`", branch = `"ddlog-4`"", `
                    "`"https://github.com/Kixiron/ddshow`", branch = `"ddlog-5`"" | Out-File $File.FullName -Encoding utf8
            }    
        }

        # cd into the directory of the generated code
        Push-Location -Path $RustDir

        # Run cargo on the generated code
        cargo build --bin "$Prefix`_cli" $ReleaseFlags | Write-Host
        $ExitCode = $LastExitCode
        Pop-Location

        # If cargo failed, exit
        if ($ExitCode -ne 0) {
            return $ExitCode
        }

    } finally {
        # Restore the rustflags env var to its previous value
        if (!$OldRustflags) {
            Remove-Item env:\RUSTFLAGS
        } elseif ($OldRustflags -Eq "") {
            $env:RUSTFLAGS = ""
        }
    }

    # Otherwise building went a-ok so we can return with a success code
    return 0
}

switch -Exact ($Command) {
    # Check 
    "check" {
        $ExitCode = 0
        Push-Location -Path $PSScriptRoot

        ddlog -i $DDlogFile --action validate | Write-Host
        $ExitCode = $LastExitCode
        Pop-Location

        Exit $ExitCode
    }

    # Build the executable
    "build" {
        $ExitCode = Build-Executable

        # If building failed, report an error
        if ($ExitCode -ne 0) {
            Write-Error "failed to build the executable for $DDlogFile, exiting"
        }
        Exit $ExitCode
    }

    # (optionally) Build and run the executable as well as (optionally) running ddshow
    "run" {
        # If `-Rebuild` was passed or there's not yet a built executable,
        # build the executable
        if ($Rebuild -or -not (Test-Path -Path $Executable -PathType Leaf)) {
            $ExitCode = Build-Executable

            # If building failed, report an error & exit
            if ($ExitCode -ne 0) {
                Write-Error "failed to build the executable for $DDlogFile, exiting"
                Exit $ExitCode
            }
        }

        # Remove the old traces if there are any
        if (Test-Path -Path $TraceDir -PathType Container) {
            Remove-Item -Path $TraceDir -Recurse
        }

        # If there's no input file then regenerate it
        if (-not (Test-Path -Path $DataFile -PathType Leaf)) {
            Regen-Input
        }

        # Load the input data
        Write-Host "Loading data file..."
        $Data = Get-Content $DataFile

        # Pipe the input data to the executable
        Write-Host "Running $Prefix with $Workers workers..."
        $Data | & $Executable --workers $Workers --timely-trace-dir $TraceDir --differential-trace-dir $TraceDir | Write-Host

        # Run ddshow on the generated trace data
        if (!$NoDDShow) {
            Push-Location -Path $CodeDir
            Write-Host "Running ddshow..."

            ddshow --workers $Workers --replay-logs $TraceDir --differential --disable-timeline

            Pop-Location
        }
    }

    # Run ddshow on ovn-reviews
    "ovn-reviews" {
        Push-Location -Path $PSScriptRoot
        Write-Host "Running ddshow..."

        ddshow --workers $Workers --replay-logs "$PSScriptRoot\ovn-reviews-trace" --differential --disable-timeline

        Pop-Location
    }

    # Regnerate the input data
    "regen" { Regen-Input }

    # This branch should be unreachable because of the `[ValidateSet()]` we have
    # on the `$Command` parameter but I'm paranoid
    default { throw "invalid command, must be one of 'check', 'build', 'run' or 'regen'" }
}
