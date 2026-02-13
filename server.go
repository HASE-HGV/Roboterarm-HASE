package main

import (
	"net/http"
	"os/exec"
	"strconv"
	"time"

	"github.com/gin-gonic/gin"
)

type motorcontrol struct { //Daten für /api/motors/...
	Steps     int    `json:"steps"`
	Clockwise bool   `json:"clockwise"`
	StepDelay string `json:"stepDelay"`
	StepStyle string `json:"stepStyle"`
}

var bAIPs []string

func setupRouter() *gin.Engine { //Erstellt und konfiguriert eine gin-Webengine
	log.Info("Erstelle API-Endpoints und HTML-Server")
	gin.SetMode(gin.ReleaseMode)

	router := gin.Default()

	router.GET("/api/sensortests", func(c *gin.Context) {
		bAIPs = append(bAIPs, c.ClientIP())
		c.Data(200, gin.MIMEPlain, []byte(":D"))
	})
	router.SetTrustedProxies(nil) //Proxies? Brauchen wir nicht
	router.GET("/api/poweroff", poweroff)
	router.GET("/api/reboot", reboot)
	router.POST("/api/motors/move", postMotors) //Motorsteuerung, funktion ist weiter unten
	//router.POST("/api/motors/stop", stopMotors)

	if !*noauth {
		router.Use(setupAuth())
	}

	router.Static("/assets", *includeFolder+"/www/assets") //Dateien in ./www sind auf dem server verfügbar, index.html als /
	router.LoadHTMLFiles(*includeFolder + "/www/index.html")

	// Routes
	router.GET("/", func(c *gin.Context) {
		c.HTML(http.StatusOK, "index.html", nil) // www/index.html
	})

	return router
}

func setupAuth() gin.HandlerFunc {
	basicAuth := gin.BasicAuth(gin.Accounts{
		"Luca-PCPasswort": "29032010",
		"Catrick":         "Bomberpilot",
		"Johannes":        "kecks",
		"gast":            "1234",
	})
	return func(c *gin.Context) {
		for counter := range bAIPs {
			if bAIPs[counter] == c.ClientIP() {
				c.Next()
				return
			}
		}
		basicAuth(c)
	}
}

func poweroff(c *gin.Context) {
	log.Info("Herunterfahren wurde von " + c.ClientIP() + " angefordert")
	c.Data(http.StatusOK, gin.MIMEPlain, []byte("Der Pi sollte jetzt herunterfahren...")) //Antworte mit code 200, content type "text/plain" und einer Nachricht
	go func() {
		time.Sleep(time.Second * 3)
		exec.Command("poweroff").Start()
	}()
}

func reboot(c *gin.Context) {
	log.Info("Neustart wurde von " + c.ClientIP() + " angefordert")
	c.Data(http.StatusOK, gin.MIMEPlain, []byte("Der Pi sollte jetzt neu starten...")) //copy+paste lol
	go func() {
		time.Sleep(time.Second * 3)
		exec.Command("reboot").Start()
	}()
}

func postMotors(c *gin.Context) { //Verarbeiten von Motor-Befehlen
	motoridStr, found := c.GetQuery("motorid") //Herausfinden der MotorID
	if !found {
		c.Data(http.StatusBadRequest, gin.MIMEPlain, []byte("Keine MotorID angegeben, du musst eine mit ?motorid=<zahl von 1 bis 4> angeben"))
		return
	}
	motorid, err := strconv.Atoi(motoridStr)
	if err != nil {
		c.Data(http.StatusBadRequest, gin.MIMEPlain, []byte("Ungültige Motorid! Muss eine Zahl von 1 bis 4 sein!"))
		return
	}
	if motorid < 1 && motorid > 4 {
		c.Data(http.StatusBadRequest, gin.MIMEPlain, []byte("Ungültige Motorid! Muss eine Zahl von 1 bis 4 sein!"))
		return
	}

	var data motorcontrol
	err = c.BindJSON(&data)
	if err != nil {
		c.Data(http.StatusBadRequest, gin.MIMEPlain, []byte(err.Error()))
		return
	}

	//Code zum Bewegen hier
	err = motor[motorid].rotateMotor(strconv.Itoa(data.Steps), data.Clockwise, data.StepStyle, data.StepDelay)

	if err == nil {
		c.Data(http.StatusOK, gin.MIMEPlain, []byte("OK"))
	} else {
		c.Data(http.StatusInternalServerError, gin.MIMEPlain, []byte(err.Error()))
	}

}
