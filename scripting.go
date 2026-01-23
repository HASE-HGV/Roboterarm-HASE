package main

type scriptingScr struct{}

var scripting scriptingScr

func (scripting scriptingScr) start() {
	log.Debug("Scripting not implemented yet!")
}

/*func (scripting scriptingScr) start() { // TODO

	master, slave, err := pty.Open()
	if err != nil {
		log.Fatal(err)
	}
	defer master.Close()
	defer slave.Close()

	if err := unix.SetNonblock(int(master.Fd()), true); err != nil {
		log.FatalStr("nonblock: " + err.Error())
	}

	_, found := os.LookupEnv("ROBOARM_SCRIPTING_PTY_PATH")
	if found {
		log.Warning("$ROBOARM_SCRIPTING_PTY_PATH ist bereits gesetzt, es wird kein neues Interface bereitgestellt.")
		log.Warning("Wenn du das ändern möchtest, gib in deine Shell 'unset ROBOARM_SCRIPTING_PTY_PATH' ein.")
		return
	}
	go func() {
		time.Sleep(time.Millisecond)
		os.Setenv("ROBOARM_SCRIPTING_PTY_PATH", slave.Name())
	}()
	log.Info("[SCRIPTING] PTY verfügbar bei " + slave.Name())
	var input string

	for {
		fmt.Fscanln(master, &input)
	}

}*/
