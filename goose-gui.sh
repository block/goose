#!/bin/bash
# Launcher script for Goose GUI
# Suppresses common GLib warnings that don't affect functionality

cd /home/alfonsodg/Devel-Local/oss/goose/ui/desktop/out/Goose-linux-x64
./Goose 2>&1 | grep -v "GLib-GObject" | grep -v "browser_main_loop"
