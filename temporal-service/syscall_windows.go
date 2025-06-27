//go:build windows
// +build windows

package main

import (
	"fmt"
	"os/exec"
	"syscall"
)

// configureSysProcAttr configures the SysProcAttr for Windows
func configureSysProcAttr(cmd *exec.Cmd) {
	// Windows doesn't support Setpgid/Pgid, so we use different approach
	cmd.SysProcAttr = &syscall.SysProcAttr{
		CreationFlags: syscall.CREATE_NEW_PROCESS_GROUP,
	}
}

// killProcessByPID kills a process on Windows
func killProcessByPID(pid int, signal syscall.Signal) error {
	// On Windows, we use taskkill command instead of syscall.Kill
	cmd := exec.Command("taskkill", "/F", "/PID", fmt.Sprintf("%d", pid))
	return cmd.Run()
}

// killProcessGroupByPID kills a process group on Windows
func killProcessGroupByPID(pid int, signal syscall.Signal) error {
	// On Windows, kill the process tree
	cmd := exec.Command("taskkill", "/F", "/T", "/PID", fmt.Sprintf("%d", pid))
	return cmd.Run()
}
