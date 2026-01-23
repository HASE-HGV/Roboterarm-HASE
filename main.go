package main

import (
	"embed"
	"flag"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"strings"
)

var includeFolder = flag.String("datadir", "./httpserver.include", "Folder that containes assets and dependencies")
var hwController = flag.String("hwCmd", *includeFolder+"/hwc/main.py", "Hardware controller exec pfad")
var noauth = flag.Bool("no-auth", false, "Disable http basic authentication")
var debugLogs = flag.Bool("debug", false, "Enable debug logs")
var disableColors = flag.Bool("disable-colors", false, "Disable colored logs")

//go:embed include
var embedDir embed.FS

func main() {
	flag.Parse()

	if !flag.Parsed() {
		fmt.Println("Error parsing arguments!")
		os.Exit(1)
	}
	log.Info("Starte...")
	log.Debug("Debug-Mode active!")
	*includeFolder = strings.TrimSuffix(*includeFolder, "/")

	if idir, err := os.Stat(*includeFolder); os.IsNotExist(err) { //Checken ob angegebenes include dir existiert, und wenn nicht, dann Dateien dahin extrahieren
		os.Mkdir(*includeFolder, 0755)
		extractIncludeDir(embedDir)
	} else if err != nil {
		log.Warning("Fehler beim Zugriff auf" + *includeFolder)
		fmt.Println(err)
		os.Exit(1)
	} else {
		if idir.IsDir() {
			log.Debug(*includeFolder + " existiert, überspringe setup...")
		} else {
			log.Debug(*includeFolder + " existiert, ist aber eine Datei!")
			os.Exit(1)
		}
	}

	_, err := os.Stat(*hwController)
	if err != nil {
		log.Error("Fehler beim finden des hwController-Skripts. Crash dump hier:")
		panic(err)
	}

	_, isInVenv := os.LookupEnv("VIRTUAL_ENV")

	if !isInVenv {
		log.Warning("ACHTUNG: Du bist in keinem python venv")
		log.Warning("Du solltest eins verwenden wenn du den standartmäßigen Hardware-Controller verwenden möchtest")
	}

	initMotorArray()        //Schaus nicht nach, DANKE!
	router := setupRouter() //Erstellt *gin.Engine und konfiguriert die API Endpoints (siehe server.go)

	go scripting.start()
	log.Info("HTTP-Server startet...")
	router.Run()
}

func extractIncludeDir(embedDir embed.FS) { // ./include in angegebenen Pfad extrahieren
	_, err := embedDir.ReadDir("include")
	if err != nil {
		log.Error("== Dieser Fehler ist wahrscheinlich beim Kompilieren entstanden ==")
		panic(err)
	}

	fs.WalkDir(embedDir, "include", func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		relPath, err := filepath.Rel("include", path)
		if err != nil {
			return err
		}
		destPath := filepath.Join(*includeFolder, relPath)
		if d.IsDir() {
			return os.MkdirAll(destPath, 0o700)
		}
		var data []byte
		if path == "include/README.txt" {
			data = []byte(`Dieser Ordner und alles darin wurde aus der roboarm httpserver binary extrahiert.
Um den Inhalt permanent zu ändern besorg dir bitte den source code des Programms,
ändere die Dateien dort im include-Ordner und kompiliere das Programm.

ÄNDERUNGEN DIE HIER VORGENOMMEN WERDEN KÖNNTEN JEDERZEIT ÜBERSCHRIEBEN WERDEN
			`)
		} else {
			data, err = fs.ReadFile(embedDir, path)
		}
		if err != nil {
			return err
		}
		log.Debug("Extrahiere " + path)
		return os.WriteFile(destPath, data, 0744)
	})
}
