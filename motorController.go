package main

import (
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"strconv"
)

type motorStruct struct {
	running bool
	id      int
}

var motor [5]motorStruct

/*
//TODO

	func (motor motorStruct) stop() {
		fmt.Println("Stoppe Motor mit ID", motor.id, "!")
		cmd := exec.Command(*hwController, "--stepstyle", "single", "--motorid", strconv.Itoa(motor.id), "--stepcount")
	}
*/

func initMotorArray() {
	motor[0] = motorStruct{id: 0} //Motor 0 sollte leer bleiben
	motor[1] = motorStruct{id: 1}
	motor[2] = motorStruct{id: 2}
	motor[3] = motorStruct{id: 3}
	motor[4] = motorStruct{id: 4}
}

func (motor motorStruct) rotateMotor(steps string, clockwise bool, stepstyle string, stepdelay string) error {
	fmt.Println("Bewege", motor.id, "| Steps:", steps, "| Uhrzeigersinn:", clockwise, "| Stil:", stepstyle, "| Verzögerung:", stepdelay)
	var clockwiseStr string = "backward"
	if clockwise {
		clockwiseStr = "forward"
	}

	if !motor.running {
		cmd := exec.Command(*hwController, "--stepstyle", stepstyle, "--motorid", strconv.Itoa(motor.id),
			"--stepcount", steps, "--delay", stepdelay, "--direction", clockwiseStr)

		go hwControllerWrapper(cmd, motor.id)
		return nil
	} else {
		return errors.New("fehler: motor dreht sich bereits")
	}

}

func hwControllerWrapper(cmd *exec.Cmd, motorid int) {
	if motor[motorid].running {
		return
	} else {
		motor[motorid].running = true
		outpipe, err := cmd.StdoutPipe()
		if err != nil {
			log.Error(err.Error())
			return
		}
		errpipe, err := cmd.StderrPipe()
		if err != nil {
			log.Error(err.Error())
			return
		}
		cmd.Start()
		go io.Copy(os.Stdout, outpipe)
		go io.Copy(os.Stderr, errpipe)
		cmd.Wait()
		motor[motorid].running = false
	}
}
