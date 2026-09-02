#!/usr/bin/env bash
#
# log_manager.sh - Executes a command, logging output to logs/build.log and terminal, without suppressing errors.
#

set -euo pipefail

LOG_FILE="logs/build.log"
mkdir -p "$(dirname "$LOG_FILE")"

# Print a separator and the command being run
echo "================================================================" | tee -a "$LOG_FILE"
echo "Running: $@" | tee -a "$LOG_FILE"
echo "Time: $(date)" | tee -a "$LOG_FILE"
echo "================================================================" | tee -a "$LOG_FILE"

# Execute the command, pipe to tee, and preserve the exit status
# Using ! prefix to avoid set -e killing the script before PIPESTATUS check
! "$@" 2>&1 | tee -a "$LOG_FILE"

EXIT_CODE=${PIPESTATUS[0]}

if [ $EXIT_CODE -ne 0 ]; then
    echo "================================================================" | tee -a "$LOG_FILE"
    echo "ERROR: Command failed with exit code $EXIT_CODE" | tee -a "$LOG_FILE"
    echo "================================================================" | tee -a "$LOG_FILE"
    exit $EXIT_CODE
fi

exit 0
