#!/bin/bash

# Define the directory path
dir_path="/opt/ubuntu/skylink/data/globe_history"

# Define the number of days to keep files
days_to_keep=2

# Print debugging information
echo "Running cleanup script"
echo "Directory path: $dir_path"
echo "Days to keep: $days_to_keep"

# Use find to locate files older than the specified number of days and print the files being deleted
find "$dir_path" -type f -mtime +$days_to_keep -exec echo "Deleting: {}" \; -exec rm {} \;

# Check the exit status of the find command
if [ $? -eq 0 ]; then
    echo "Find command executed successfully"
else
    echo "Find command failed"
fi

# Optionally, you can add a log message to record the cleanup
echo "$(date): Cleaned up files older than $days_to_keep days in $dir_path" >> /var/log/cleanup.log

