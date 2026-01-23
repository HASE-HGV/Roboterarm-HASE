package main

import (
	"fmt"
	"io"
	"os"
	"time"
)

type colors struct {
}

var color colors

type logStr struct{}

var log logStr

func (color colors) reset() string { //color:normal
	if *disableColors {
		return ""
	} else {
		return "\x1b[0m"
	}
}
func (color colors) red() string { //color:red
	if *disableColors {
		return ""
	} else {
		return "\x1b[31m"
	}
}
func (color colors) green() string { //color:green
	if *disableColors {
		return ""
	} else {
		return "\x1b[32m"
	}
}
func (color colors) yellow() string { //color:yellow/orange
	if *disableColors {
		return ""
	} else {
		return "\u001b[33m"
	}
}
func (color colors) white() string { //color:white
	if *disableColors {
		return ""
	} else {
		return "\u001b[37m"
	}
}

func (log logStr) Info(message any) error {
	return log.Log(color.green(), "INFO", os.Stdout, message)
}

func (log logStr) Warning(message any) error {
	return log.Log(color.yellow(), "WARN", os.Stderr, message)
}

func (log logStr) Error(message any) error {
	return log.Log(color.red(), "ERR", os.Stderr, message)
}

func (log logStr) Fatal(err error) {
	log.Log(color.red(), "FATAL", os.Stderr, "A fatal error occurred. The program will now exit.")
	log.Log(color.red(), "FATAL", os.Stderr, "== A detailed crash dump may be seen below ==")
	fmt.Print(color.red())
	panic(err.Error() + color.reset())
}

func (log logStr) FatalStr(err string) {
	log.Log(color.red(), "FATAL", os.Stderr, "A fatal error occurred. The program will now exit.")
	log.Log(color.red(), "FATAL", os.Stderr, "== A detailed crash dump may be seen below ==")
	fmt.Print(color.red())
	panic(err + color.reset())
}

func (log logStr) Debug(message any) error {
	if *debugLogs {
		return log.Log(color.white(), "DEBUG", os.Stdout, message)
	}
	return nil
}

func (log logStr) Log(useColor string, level string, dest io.Writer, message any) error {
	_, err := fmt.Fprint(dest, useColor+"["+time.Now().Local().Format(time.TimeOnly)+"] "+"["+level+"]", " | ", message)
	fmt.Fprintln(dest, color.reset())
	return err
}
