#!/bin/bash

# Get the previous day's date in YYYY/MM/DD format
PREV_DATE=$(date -d "yesterday" '+%Y/%m/%d')

# Construct the directory path
DIR_TO_DELETE="/home/ubuntu/skylink/data/globe_history/$PREV_DATE"

# Check if the directory exists and delete it if it does
if [ -d "$DIR_TO_DELETE" ]; then
    sudo rm -rf "$DIR_TO_DELETE"
    echo "Deleted: $DIR_TO_DELETE"
else
    echo "Directory does not exist: $DIR_TO_DELETE"
fi
