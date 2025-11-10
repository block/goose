#!/bin/bash
# Launcher script for goose GUI
# Suppresses common GLib warnings that don't affect functionality

cd /home/alfonsodg/Devel-Local/oss/goose/ui/desktop/out/goose-linux-x64
./goose 2>&1 | grep -v "GLib-GObject" | grep -v "browser_main_loop"
