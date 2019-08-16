/**
 * Name: Muses rotary encoders and buttons
 * 
 * Description: 
 *     Simple Arduino program to handle the 4 rotary encoders and 4 buttons on the Muses hardware
 *     instrument.
 * 
 * Copyright: Benedict R. Gaster, 2019
 * 
 * 
 *  Pins
 *  
 *  Rotary encoder:
 *  
 *  On the device encoders are installed as:
 *  
 *        RE3 | RE2 | RE1 | RE0
 *  
 *    E0 encoder input, which attached to interrupt
 *    E1 encoder input, not attached to interrupt
 *  
 *  
 *              E0/Int     E1   
 *      RE0 -   0/INT2     4
 *      RE1 -   1/INT3     6
 *      RE2 -   3/INT0     10  
 *      RE3 -   2/INT1     8  
 *      
 *  Buttons:
 *  
 *       B3 | B2 | B1 | B0
 *       
 *       B0 - A5
 *       B1 - A0
 *       B2 - A2
 *       B3 - A3
 */
 
//----------------------------------------------------------------------------------------
// first rotary encoder stuff
//----------------------------------------------------------------------------------------

// struct to hold information about a single encoder
struct Rotary {
  // encoder pin, interrupt enabled
  byte pinA;
  // encoder pin, no interrupt
  byte pinB;
  // pin interrupt
  byte intp;
  void (*handler)(void);
  // current position of encoder
  int pos;
  // has state of encoder changed
  bool changed;
  // OSC message to be sent when encoder state changes
  char msg[6];
};

// predefine handlers
void handler0();
void handler1();
void handler2();
void handler3();

// allocate encoders
Rotary rotarys[] = {
    { 0, 4, 2, &handler0, 0, false, "/e/0 " },
    { 1, 6, 3, &handler1, 0, false, "/e/1 " },
    { 3, 10, 0, &handler2, 0, false, "/e/2 " },
    { 2, 8, 1, &handler3, 0, false, "/e/3 " }
};

// number of encoders
#define NUMENCODERS ((sizeof(rotarys)/sizeof(*rotarys)))

// macro to make interrupt handler(s) for each encoder
#define mkHandler(num) void handler##num() { \
  noInterrupts(); \
 \ 
  if (digitalRead(rotarys[num].pinA) == digitalRead(rotarys[num].pinB)) { \
    rotarys[num].pos = -1; \
  } \
  else { \
    rotarys[num].pos = 1; \
  } \
  rotarys[num].changed = true; \
\ 
  interrupts(); \
}

// now define the handlers
mkHandler(0)
mkHandler(1)
mkHandler(2)
mkHandler(3)

//----------------------------------------------------------------------------------------
// now some button stuff
//----------------------------------------------------------------------------------------

#define DEBOUNCE 10  // button debouncer, how many ms to debounce, 5+ ms is usually plenty

// struct to hold information for a single button
struct Button {
  // button pin
  byte pin;
  // current state of button
  byte state;
  // button pressed
  byte pressed;
  // OSC message when button pressed
  char msgOn[10];
  // OSC message when button released
  char msgOff[10];
};

// allocate buttons
Button buttons[] = {
  { A5, 1, 0, "/b/0 127\n", "/b/0 0\n" },
  { A0, 1, 0, "/b/1 127\n", "/b/1 0\n" },
  { A2, 1, 0, "/b/2 127\n", "/b/2 0\n" },
  { A3, 1, 0, "/b/3 127\n", "/b/3 0\n" } 
};
 
// number of buttons
#define NUMBUTTONS ((sizeof(buttons)/sizeof(*buttons)))

void setup() {

  // set up serial port for OSC messages
  Serial.begin(9600);

  // initialize encoders
  for (byte i=0; i< NUMENCODERS; i++) {
    pinMode(rotarys[i].pinA, INPUT_PULLUP);
    pinMode(rotarys[i].pinB, INPUT_PULLUP);
    attachInterrupt(rotarys[i].intp, rotarys[i].handler, CHANGE);
  }

  // initialise buttons
  for (byte i=0; i< NUMBUTTONS; i++) {
    pinMode(buttons[i].pin, INPUT);
  }
}

// process any button interaction
void handleButtons()
{
  static long lasttime;

  // lazy debouncing, handle all buttons on one shared notion of time...
  if (millis() < lasttime) {
     lasttime = millis(); // we wrapped around, lets just try again
  }
 
  if ((lasttime + DEBOUNCE) > millis()) {
    return; // not enough time has passed to debounce
  }
  
  // ok we have waited DEBOUNCE milliseconds, lets reset the timer
  lasttime = millis();

  // now process any button presses/releases
  for (byte i = 0; i < NUMBUTTONS; i++) {
    int reading = digitalRead(buttons[i].pin);
    if (buttons[i].state != reading) {
      buttons[i].state = reading;

      // is button pressed
      if (buttons[i].state == HIGH) {
            Serial.print(buttons[i].msgOn);  
      }
      else { // or released
            Serial.print(buttons[i].msgOff);
      }
     
    }
  }
}

// process any encoder movement
void handleEncoders() {
  for (byte i = 0; i < NUMENCODERS; i++) {
    if (rotarys[i].changed) {
      Serial.print(rotarys[i].msg);
      Serial.print(rotarys[i].pos, DEC);
      Serial.print("\n");
      rotarys[i].changed = false;
    }
  }
}

// process buttons and encoders, over and over
void loop() {
   handleButtons();
   handleEncoders();
}
