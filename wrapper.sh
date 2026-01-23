#! /bin/bash
# Dieses Skript wird ausgeführt wenn SystemD roboarm.service startet!
# Es soll prüfen ob ein Ordner namens roboarm_updates existiert und wenn ihn in /roboarm verschieben und die binary /roboarm/core ausführen
# Du kannst es auch verwenden um Befehle auf dem Pi auszuführen, pack sie weiter unten hin

go build -o core # Kommentieren, um zu verhindern dass das Programm vor dem Starten neu kompiliert wird!
rm -rf httpserver.include # Kommentieren, um zu verhindern dass das assets dir bei jedem Neustart des Programms neu erstellt wird.
CORE_BIN="./core"
PORT=80 #Port auf dem der Httpserver läuft, 80 ist Standart

echo Roboterarm controller wrapper script v0.3

#Hier kannst du deine Befehle einfügen, du kannst sleep 20;poweroff dahinterschreiben damit der pi danach wieder ausgeht.
#(und ja das sleep ist nötig weil diese Befehle noch beim Hochfahren ausgeführt werden und Herunterfahren da noch nicht geht.)


echo Der Port ist $PORT
echo Starte $CORE_BIN # CORE_BIN ist in der SystemD Unit-Datei definiert, um es zu ändern kannst du die Zeile drüber auskommentieren
$CORE_BIN
CORE_STATUSCODE = $?
echo core ist mit code $CORE_STATUSCODE beendet worden.
exit $CORE_STATUSCODE