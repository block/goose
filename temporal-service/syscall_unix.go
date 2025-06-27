//go:build !windows
// +build !windows

package main

import (
	"os/exec"
	"syscall"
)

// configureSysProcAttr configures the SysProcAttr for Unix-like systems
func configureSysProcAttr(cmd *exec.Cmd) {
	cmd.SysProcAttr = &syscall.SysProcAttr{
		Setpgid: true, // Create new process group
		Pgid:    0,    // Use process ID as group ID
	}
}

// killProcessByPID kills a process using Unix syscalls
func killProcessByPID(pid int, signal syscall.Signal) error {
	return syscall.Kill(pid, signal)
}

// killProcessGroupByPID kills a process group using Unix syscalls
func killProcessGroupByPID(pid int, signal syscall.Signal) error {
	return syscall.Kill(-pid, signal)
}
