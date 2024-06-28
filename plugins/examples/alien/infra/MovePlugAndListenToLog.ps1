# Define source and destination paths
$sourcePath = "C:\Users\97250\Desktop\rust\nih-plug\target\bundled\Alien.vst3\Contents\x86_64-win\alien.vst3"
$destinationPath = "C:\Program Files\Common Files\VST3\alien.vst3"
$logFilePath = "output.log"

# Function to check for administrative privileges
function Test-Admin {
    $currentUser = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    return $currentUser.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

# Function to write log messages
function Write-Log {
    param (
        [string]$message
    )
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $logMessage = "$timestamp - $message"
    Write-Output $logMessage
    Add-Content -Path $logFilePath -Value $logMessage
}

# Run the script as admin if not already running as admin
if (-not (Test-Admin)) {
    $arguments = "& '" + $myInvocation.MyCommand.Definition + "'"
    Start-Process powershell.exe -ArgumentList $arguments -Verb RunAs
    exit
}


Write-Log "Copying plugin file from $sourcePath to $destinationPath."
try {
    Copy-Item -Path $sourcePath -Destination $destinationPath -Force
    Write-Log "Plugin copied successfully."
} catch {
    Write-Log "Error copying Plugin: $_"
}

# Remove existing log file and create a new one
Write-Log "Removing existing log file if it exists."
if (Test-Path -Path $logFilePath) {
    try {
        Remove-Item -Path $logFilePath -Force
        Write-Log "Existing log file removed."
    } catch {
        Write-Log "Error removing log file: $_"
    }
}

Write-Log "Creating new log file."
try {
    New-Item -Path $logFilePath -ItemType File -Force
    Write-Log "New log file created."
} catch {
    Write-Log "Error creating new log file: $_"
}

Write-Log "Starting to monitor the log file."

# Execute the Get-Content command on output.log with -Wait
Get-Content $logFilePath -Wait
