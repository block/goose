package main

import (
	"log"
	"os"
	"os/signal"

	"opamp-server/data"
	"opamp-server/server"
)

var logger = log.New(log.Default().Writer(), "[MAIN] ", log.Default().Flags()|log.Lmsgprefix|log.Lmicroseconds)

func main() {
	logger.Println("OpAMP Server starting...")

	// Create and start the OpAMP server
	opampSrv := server.NewServer(&data.AllAgents)
	opampSrv.Start()

	logger.Println("OpAMP Server running...")

	// Wait for interrupt signal
	interrupt := make(chan os.Signal, 1)
	signal.Notify(interrupt, os.Interrupt)
	<-interrupt

	logger.Println("OpAMP Server shutting down...")
	opampSrv.Stop()
}
