@echo off
set GOOS=linux
set GOARCH=arm
set ARM=6
go build -o core
echo Compiled to file core
exit /b