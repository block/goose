package server

import (
	"context"
	"log"
)

// OpAMPLogger wraps a standard logger to implement the OpAMP Logger interface
type OpAMPLogger struct {
	logger *log.Logger
}

func NewOpAMPLogger(logger *log.Logger) *OpAMPLogger {
	return &OpAMPLogger{logger: logger}
}

func (l *OpAMPLogger) Debugf(_ context.Context, format string, v ...interface{}) {
	l.logger.Printf("[DEBUG] "+format, v...)
}

func (l *OpAMPLogger) Errorf(_ context.Context, format string, v ...interface{}) {
	l.logger.Printf("[ERROR] "+format, v...)
}

func (l *OpAMPLogger) Warnf(_ context.Context, format string, v ...interface{}) {
	l.logger.Printf("[WARN] "+format, v...)
}

func (l *OpAMPLogger) Infof(_ context.Context, format string, v ...interface{}) {
	l.logger.Printf("[INFO] "+format, v...)
}

func (l *OpAMPLogger) Debug(_ context.Context, msg string) {
	l.logger.Printf("[DEBUG] %s", msg)
}

func (l *OpAMPLogger) Error(_ context.Context, msg string) {
	l.logger.Printf("[ERROR] %s", msg)
}

func (l *OpAMPLogger) Warn(_ context.Context, msg string) {
	l.logger.Printf("[WARN] %s", msg)
}

func (l *OpAMPLogger) Info(_ context.Context, msg string) {
	l.logger.Printf("[INFO] %s", msg)
}

func (l *OpAMPLogger) Printf(format string, v ...interface{}) {
	l.logger.Printf(format, v...)
}

func (l *OpAMPLogger) Print(v ...interface{}) {
	l.logger.Print(v...)
}

func (l *OpAMPLogger) Println(v ...interface{}) {
	l.logger.Println(v...)
}

func (l *OpAMPLogger) Fatal(v ...interface{}) {
	l.logger.Fatal(v...)
}

func (l *OpAMPLogger) Fatalf(format string, v ...interface{}) {
	l.logger.Fatalf(format, v...)
}

func (l *OpAMPLogger) Fatalln(v ...interface{}) {
	l.logger.Fatalln(v...)
}

func (l *OpAMPLogger) Panic(v ...interface{}) {
	l.logger.Panic(v...)
}

func (l *OpAMPLogger) Panicf(format string, v ...interface{}) {
	l.logger.Panicf(format, v...)
}

func (l *OpAMPLogger) Panicln(v ...interface{}) {
	l.logger.Panicln(v...)
}
