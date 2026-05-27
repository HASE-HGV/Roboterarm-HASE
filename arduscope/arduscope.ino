#include <Arduino.h>

const byte INTERRUPT_PIN = 21;

volatile unsigned long lastPulseTime = 0;
volatile unsigned long currentPeriod = 0;
volatile boolean newPulse = false;

unsigned long pulseCount = 0;
double sumPeriods = 0.0;
double sumSquarePeriods = 0.0;
double minPeriod = 99999999.0;
double maxPeriod = 0.0;

const unsigned long MIN_PERIOD_US = 450;

// Escape-Sequenzen für Minicom
#define RST "\033[0m"
#define CYN "\033[36m"
#define YEL "\033[33m"
#define GRN "\033[32m"
#define BLU "\033[34m"
#define MAG "\033[35m"

unsigned long displayInterval = 500;
bool compactMode = false;
bool langEnglish = false;
bool frozen = false;
bool colorEnabled = true;

const char* txt( const char* de, const char* en ) {
  return langEnglish ? en : de;
}

// Funktion zum Leeren des Minicom-Bildschirms
void clearScreen() {
  Serial.print("\033[2J");    // Löscht den gesamten Bildschirm
  Serial.print("\033[H");     // Setzt den Cursor oben links hin
}

void pulseISR() {
  unsigned long now = micros();
  unsigned long period = now - lastPulseTime;
  if (period > MIN_PERIOD_US) {
    currentPeriod = period;
    lastPulseTime = now;
    newPulse = true;
  } else if (lastPulseTime == 0) {
    lastPulseTime = now;
  }
}

void resetStats() {
  pulseCount = 0;
  sumPeriods = 0.0;
  sumSquarePeriods = 0.0;
  minPeriod = 99999999.0;
  maxPeriod = 0.0;
}

void printHelp() {
  Serial.println();
  if (colorEnabled) Serial.print(CYN);
  Serial.println(txt("=== BEFEHLE ===", "=== COMMANDS ==="));
  if (colorEnabled) Serial.print(RST);
  Serial.println(txt("  h  - Diese Hilfe", "  h  - This help"));
  Serial.println(txt("  c  - Kompaktmodus umschalten", "  c  - Toggle compact mode"));
  Serial.println(txt("  r  - Statistik zurücksetzen", "  r  - Reset statistics"));
  Serial.println(txt("  e  - Sprache umschalten (DE/EN)", "  e  - Toggle language (DE/EN)"));
  Serial.println(txt("  f  - Display anhalten/fortsetzen", "  f  - Freeze/resume display"));
  Serial.println(txt("  t  - Farben umschalten", "  t  - Toggle colors"));
  Serial.println(txt("  +  - Schnellere Aktualisierung", "  +  - Faster update"));
  Serial.println(txt("  -  - Langsamere Aktualisierung", "  -  - Slower update"));
  Serial.println(txt("  s  - Status anzeigen", "  s  - Show status"));
  Serial.println(txt("  ?  - Diese Hilfe", "  ?  - This help"));
  Serial.println();
}

void printStatus() {
  Serial.print(txt("Modus: ", "Mode: "));
  Serial.print(compactMode ? txt("KOMPAKT", "COMPACT") : txt("AUSFÜHRLICH", "VERBOSE"));
  Serial.print(txt(" | Sprache: ", " | Lang: "));
  Serial.print(langEnglish ? "EN" : "DE");
  Serial.print(txt(" | Farbe: ", " | Color: "));
  Serial.print(colorEnabled ? txt("AN", "ON") : txt("AUS", "OFF"));
  Serial.print(txt(" | Intervall: ", " | Interval: "));
  Serial.print(displayInterval);
  Serial.print(txt(" ms | ", " ms | "));
  Serial.println(frozen ? txt("GEFROREN", "FROZEN") : txt("LIVE", "LIVE"));
}

void handleSerial() {
  if (Serial.available() <= 0) return;
  char c = Serial.read();
  switch (c) {
    case 'h': case '?': clearScreen(); printHelp(); break;
    case 'c': compactMode = !compactMode; clearScreen(); printStatus(); break;
    case 'r': resetStats(); clearScreen(); Serial.println(txt("Statistik zurückgesetzt.", "Statistics reset.")); break;
    case 'e': langEnglish = !langEnglish; clearScreen(); printStatus(); break;
    case 'f': frozen = !frozen; Serial.println(frozen ? txt("GEFROREN", "FROZEN") : txt("LIVE", "LIVE")); break;
    case 't': colorEnabled = !colorEnabled; clearScreen(); printStatus(); break;
    case '+':
      if (displayInterval > 50) displayInterval -= 50;
      clearScreen();
      Serial.print(txt("Intervall: ", "Interval: ")); Serial.print(displayInterval); Serial.println(" ms");
      break;
    case '-':
      if (displayInterval < 5000) displayInterval += 50;
      clearScreen();
      Serial.print(txt("Intervall: ", "Interval: ")); Serial.print(displayInterval); Serial.println(" ms");
      break;
    case 's': clearScreen(); printStatus(); break;
  }
}

void setup() {
  Serial.begin(115200);
  pinMode(INTERRUPT_PIN, INPUT_PULLUP);
  attachInterrupt(digitalPinToInterrupt(INTERRUPT_PIN), pulseISR, RISING);
  
  clearScreen(); 
  Serial.println(txt("Arduscope gestartet. 'h' oder '?' fuer Hilfe.", "Arduscope started. 'h' or '?' for help."));
  Serial.println(txt("Warte auf Signal...", "Waiting for signal..."));
}

void loop() {
  handleSerial();

  noInterrupts();
  boolean pulseAvailable = newPulse;
  unsigned long periodCopy = currentPeriod;
  newPulse = false;
  interrupts();

  if (pulseAvailable && periodCopy > 0) {
    pulseCount++;

    if (periodCopy < minPeriod) minPeriod = periodCopy;
    if (periodCopy > maxPeriod) maxPeriod = periodCopy;

    sumPeriods += periodCopy;
    sumSquarePeriods += ((double)periodCopy * (double)periodCopy);

    static unsigned long lastDisplay = 0;
    if (!frozen && millis() - lastDisplay > displayInterval) {
      lastDisplay = millis();

      double avgPeriod = sumPeriods / pulseCount;
      double variance = (sumSquarePeriods / pulseCount) - (avgPeriod * avgPeriod);
      if (variance < 0) variance = 0;
      double stdDevPeriod = sqrt(variance);

      // Der Bildschirm wird jetzt vor jedem Druck-Intervall komplett geleert
      clearScreen(); 

      if (compactMode) {
        if (colorEnabled) Serial.print(CYN);
        Serial.print("["); Serial.print(pulseCount); Serial.print("]");
        if (colorEnabled) Serial.print(RST " " YEL);
        Serial.print(" "); Serial.print((double)periodCopy, 1);
        if (colorEnabled) Serial.print(RST);
        Serial.print(txt(" us ", " us "));
        if (colorEnabled) Serial.print(GRN);
        Serial.print(txt("avg:", "avg:"));
        Serial.print(avgPeriod, 1);
        if (colorEnabled) Serial.print(RST " " BLU);
        Serial.print(txt(" sd:", " sd:"));
        Serial.print(stdDevPeriod, 1);
        if (colorEnabled) Serial.print(RST " " MAG);
        Serial.print(txt(" min:", " min:"));
        Serial.print(minPeriod, 1);
        Serial.print(txt(" max:", " max:"));
        Serial.print(maxPeriod, 1);
        if (colorEnabled) Serial.print(RST);
        Serial.println(txt(" us", " us"));
      } else {
        if (colorEnabled) Serial.print(CYN);
        Serial.print(txt("--- MESSWERTE", "--- VALUES"));
        Serial.print(txt(" (Pulse: ", " (Pulses: "));
        Serial.print(pulseCount); Serial.println(") ---");
        if (colorEnabled) Serial.print(RST);
        Serial.print(txt("Aktuelle Periode:  ", "Current period:     "));
        if (colorEnabled) Serial.print(YEL);
        Serial.print((double)periodCopy, 1); Serial.println(txt(" us", " us"));
        if (colorEnabled) Serial.print(RST);
        Serial.print(txt("Durchschnitt (Avg):", "Average (Avg):      "));
        if (colorEnabled) Serial.print(GRN);
        Serial.print(avgPeriod, 1); Serial.println(txt(" us", " us"));
        if (colorEnabled) Serial.print(RST);
        Serial.print(txt("Abweichung (StdDev):", "Std deviation:      "));
        if (colorEnabled) Serial.print(BLU);
        Serial.print(stdDevPeriod, 1); Serial.println(txt(" us", " us"));
        if (colorEnabled) Serial.print(RST);
        Serial.print(txt("Minimale Periode:  ", "Minimum period:     "));
        if (colorEnabled) Serial.print(MAG);
        Serial.print(minPeriod, 1); Serial.println(txt(" us", " us"));
        if (colorEnabled) Serial.print(RST);
        Serial.print(txt("Maximale Periode:  ", "Maximum period:     "));
        if (colorEnabled) Serial.print(MAG);
        Serial.print(maxPeriod, 1); Serial.println(txt(" us", " us"));
        if (colorEnabled) Serial.print(RST);
        Serial.println();
      }
    }
  }
}
