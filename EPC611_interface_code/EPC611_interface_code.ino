#include <SPI.h>
//#include </home/erhannis/.arduino15/packages/esp32/hardware/esp32/2.0.9/libraries/SPI/src/SPI.cpp>

// Define ALTERNATE_PINS to use non-standard GPIO pins for SPI bus

/*
So, I coulda sworn the initial readings off the first chip I tried were non-garbage.  I...wait.
...
...I have to make each message a separate transaction???  Ok, whatever, at least I'm getting something??
*/

#ifdef ALTERNATE_PINS
  #define HSPI_MISO   26
  #define HSPI_MOSI   27
  #define HSPI_SCLK   25
  #define HSPI_SS     32
#else
  // #define HSPI_MISO   MISO
  // #define HSPI_MOSI   MOSI
  // #define HSPI_SCLK   SCK
  // #define HSPI_SS     SS
  #define HSPI_MISO   12
  #define HSPI_MOSI   13
  #define HSPI_SCLK   14
  #define HSPI_SS     15
#endif

#define DATA_RDY 27

#define SPI_MODE SPI_MODE0
#define AUTO_INIT 1
#define AUTO_SHUTTER 1

const uint16_t BS_STARTUP[] = {
  //DUMMY Load sequencer
};

const uint16_t BS_SETTNGS_1[] = {
  0x8100, // Page select 1
  0x5A00, // Adjust 1
  0x8500, // Page select 5
  0x4B00, // Adjust 2
  0x0000  // NOP //THINK Skip nop?
};

// Only for WAFER ID < 13
const uint16_t BS_SETTNGS_2[] = {
  0x8400, // Page select 4
  0x481F, // Adjust 3
  0x8500, // Page select 5
  0x4E01, // Adjust 4
  0x8600, // Page select 6
  0x5162, // Adjust 5
  0x0000  // NOP //DITTO
};

const uint16_t BS_SET_MODE_GIM[] = {
  0x8400, // Page select 4
  0x52C0, // Set modulation selection
  0x5523, // Set 8x8 greyscale imager mode
  0x0000  // NOP //DITTO  
};

const uint16_t BS_SET_MODULATION_FREQUENCY[] = {
  0x8400, // Page select 4
  0x4501, // Set mod. freq. to 10MHz , Is also integration time base
  0x0000  // NOP //DITTO  
};

const uint16_t BS_SET_INTEGRATION_TIME[] = {
  0x8500, // Page select 5
  // Set int. time 1.6384ms
  0x4000, // Integration time multiplier, high byte
  0x4101, // Integration time multiplier, low byte (lowest number = 1)
  0x4204, // Integration length, high byte //RAINY Permit control
  0x43FF, // Integration length, low byte
  0x0000  // NOP //DITTO  
};

const uint16_t BS_START_MEASUREMENT[] = {
  0x8200, // Page select 2
  0x5801, // Set TRIGGER, start measurement
  0x0000  // NOP //DITTO  
};

const uint16_t BS_READ_WAFER_ID[] = {
  0x8700,
  0x3600,
  0x3700,
  0x3800,
  0x3900,
  0x0000
};

const uint16_t BS_READ_2ROW[] = {
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x2C00,
  0x0000
};

static const int spiClk = 16000000;

//uninitalised pointers to SPI objects
SPIClass * hspi = NULL;

void setup() {
  Serial.begin(115200);
  delay(2000);
  Serial.println(MISO);
  Serial.print("MISO ");
  Serial.println(HSPI_MISO);
  Serial.print("MOSI ");
  Serial.println(HSPI_MOSI);
  Serial.print("SCK  ");
  Serial.println(HSPI_SCLK);
  Serial.print("SS   ");
  Serial.println(HSPI_SS);

  // Serial.println("Delaying 5 seconds");
  // delay(5000);
  // Serial.println("Done delaying");

  //initialise instance of the SPIClass attached to HSPI
  hspi = new SPIClass(HSPI);
  
  //clock miso mosi ss
#ifndef ALTERNATE_PINS
  //initialise hspi with default pins
  //SCLK = 14, MISO = 12, MOSI = 13, SS = 15
  hspi->begin();
#else
  //alternatively route through GPIO pins
  hspi->begin(HSPI_SCLK, HSPI_MISO, HSPI_MOSI, HSPI_SS); //SCLK, MISO, MOSI, SS
#endif

  //set up slave select pins as outputs as the Arduino API
  //doesn't handle automatically pulling SS low
  pinMode(HSPI_SS, OUTPUT); //HSPI SS
  pinMode(DATA_RDY, INPUT);
  
  wait_ready();

  if (AUTO_INIT) {
    processCommand("init");
  }
}

String incomingCommand = "";
void readSerialCommand() {
  if (Serial.available() > 0) {
    char incomingChar = Serial.read();
    if (incomingChar == '\r') {
      // Nothing, ignore
    } else if (incomingChar == '\n') {
      processCommand(incomingCommand);
      incomingCommand = "";
    } else {
      incomingCommand += incomingChar;
    }
  }
}

//chatgpt
long int parseHexString(String hexString) {
  //THINK What about .c_str?
  char charBuffer[hexString.length() + 1]; // Create a char array to hold the C-style string
  hexString.toCharArray(charBuffer, sizeof(charBuffer)); // Convert the String to a char array
  char *endPtr; // Pointer to the first invalid character after the parsed number
  long int result = strtol(charBuffer, &endPtr, 16); // Convert the char array to a long int
  return result;
}

int16_t convertSigned12to16(int16_t x) {
  bool isNegative = (x & 0x800) != 0;
  int16_t signExtendedValue = isNegative ? (x | 0xF000) : (x & 0xFFF);
  return signExtendedValue;
}

inline int16_t s12_s16(int16_t x) {
  return ((x & 0x800) * 0b11110) | x; //THINK Is the multiplication slower than the conditional?
  //CHECK Also, is this right?
}

void printFrame(int8_t frame[]) { //THINK assumes 8x8
  for (int y = 0; y < 8; y++) {
    for (int x = 0; x < 8; x++) {
      Serial.printf("%02X ", (uint8_t)frame[(y*8)+x]);
    }
    Serial.println();
  }
}

const int PALETTE_N = 7;
const char PALETTE[] = {'#','I','o','*',':','-',' '};
char paintMap[256];
// void scaleForFrame(int8_t frame[]) {
// }
void printFrameScaled(int8_t frame[]) { //THINK assumes 8x8
  int8_t min = 0x7F;
  int8_t max = 0x80;
  for (int i = 0; i < 8*8; i++) {
    int8_t f = frame[i];
    if (f < min) {
      min = f;
    }
    if (f > max) {
      max = f;
    }
  }
  Serial.printf("minmax %d %d\n", min, max);

  int rangeAB = max - min + 1;
  for (int i = 0; i < 8*8; i++) {
  }
  for (int y = 0; y < 8; y++) {
    for (int x = 0; x < 8; x++) {
      int p = ((frame[(y*8)+x] - min) * (PALETTE_N-1) + rangeAB / 2) / rangeAB;  
      //Serial.printf("%d", p);
      Serial.print(PALETTE[p]);
    }
    Serial.println();
  }
}

void processCommand(String command) {
  // Compare the received command with predefined commands
  if (command == "h" || command == "help") {
    Serial.println("h/help, id, t####, init, dr, c/s/shutter"); //PERIODIC Keep up to date
  } else if (command == "id") {
    read_wafer_id();
  } else if (command.length() == 5 && command[0] == 't') {
    uint16_t c = parseHexString(command.substring(1, 5));
    print_exchange(c);
  } else if (command == "init") {
    //DUMMY Probably do this automatically
    //DUMMY Load sequencer
    print_exchange_buffer(BS_SETTNGS_1, 0, sizeof(BS_SETTNGS_1) / sizeof(BS_SETTNGS_1[0]));
    // skip settings 2, at least for my chips //DUMMY will we ever need to deal with wafer < 13?
    print_exchange_buffer(BS_SET_MODE_GIM, 0, sizeof(BS_SET_MODE_GIM) / sizeof(BS_SET_MODE_GIM[0]));
    print_exchange_buffer(BS_SET_MODULATION_FREQUENCY, 0, sizeof(BS_SET_MODULATION_FREQUENCY) / sizeof(BS_SET_MODULATION_FREQUENCY[0]));
    print_exchange_buffer(BS_SET_INTEGRATION_TIME, 0, sizeof(BS_SET_INTEGRATION_TIME) / sizeof(BS_SET_INTEGRATION_TIME[0]));
  } else if (command == "c" || command == "s" || command == "shutter") {
    print_exchange(0x8200);
    print_exchange(0x5801); // Set trigger
    
    int8_t frame[8*8]; //DUMMY int16
    uint16_t row_buf[24+1];
    uint8_t row2[24];

    // Read frame
    Serial.println("frame:");
    unsigned long delay = 0;
    unsigned long ds = 0;
    unsigned long start = micros();
    for (int i = 3; i >= 0; i--) {
      // Wait for data ready
      ds = micros();
      while (!digitalRead(DATA_RDY));
      delay += micros()-ds;

      exchange_buffer(BS_READ_2ROW, 0, sizeof(BS_READ_2ROW) / sizeof(BS_READ_2ROW[0]), row_buf, 0);
      //DUMMY I'm dropping the least significant 4 bits
      for (int j = 0; j < 4; j++) {
        int k = j*3;
        frame[((  i)*8)+(j*2  )] = (int8_t)(row_buf[k+1] & 0x00FF); //DUMMY int16
        frame[((  i)*8)+(j*2+1)] = (int8_t)(row_buf[k+3] & 0x00FF);
      }
      for (int j = 0; j < 4; j++) {
        int k = (j+4)*3;
        frame[((7-i)*8)+(j*2  )] = (int8_t)(row_buf[k+1] & 0x00FF);
        frame[((7-i)*8)+(j*2+1)] = (int8_t)(row_buf[k+3] & 0x00FF);
      }
    }
    unsigned long stop = micros();
    Serial.printf("micros elapsed: %ld\n", stop-start);
    Serial.printf("micros delay: %ld\n", delay);
    Serial.println();
    printFrameScaled(frame);
    Serial.println();
  } else if (command == "dr") {
    int dataRdy = digitalRead(DATA_RDY);
    Serial.printf("dataRdy: %d\n", dataRdy);
  } else {
    Serial.println("Invalid command"); // Command not recognized
  }
}


// the loop function runs over and over again until power down or reset
void loop() {
  if (AUTO_SHUTTER) {
    processCommand("s");
  } else {
    readSerialCommand();
  }
  delay(250);
}

byte count = 0;

int print_exchange(int tx) {
  hspi->beginTransaction(SPISettings(spiClk, MSBFIRST, SPI_MODE));
  digitalWrite(HSPI_SS, LOW);
  uint16_t rx = hspi->transfer16(tx);
  digitalWrite(HSPI_SS, HIGH);
  hspi->endTransaction();
  Serial.printf("tx/rx %04X/%04X\n", tx, rx);
  return rx;
}

uint16_t exchangeBuffer[256];
void print_exchange_buffer(const uint16_t tx[], int offset, int count) {
  hspi->beginTransaction(SPISettings(spiClk, MSBFIRST, SPI_MODE));
  for (int i = 0; i < count; i++) {
    digitalWrite(HSPI_SS, LOW);
    exchangeBuffer[i] = hspi->transfer16(tx[offset+i]);
    digitalWrite(HSPI_SS, HIGH); // Apparently I have to toggle nss between words
  }
  hspi->endTransaction();
  for (int i = 0; i < count; i++) {
    Serial.printf("tx/rx %04X/%04X\n", tx[offset+i], exchangeBuffer[i]);
  }
  Serial.println();
}
void exchange_buffer(const uint16_t tx[], int tx_offset, int count, uint16_t rx[], int rx_offset) {
  hspi->beginTransaction(SPISettings(spiClk, MSBFIRST, SPI_MODE));
  for (int i = 0; i < count; i++) {
    digitalWrite(HSPI_SS, LOW);
    rx[rx_offset+i] = hspi->transfer16(tx[tx_offset+i]);
    digitalWrite(HSPI_SS, HIGH); // Apparently I have to toggle nss between words
  }
  hspi->endTransaction();
}

void wait_ready() {
  //DUMMY Actually look at response
  Serial.println("delaying for ready....");
  for (int i = 0; i < 30; i++) {
    uint16_t rx = print_exchange(0x0000);
    if (rx == 0x0000) {
      Serial.println("Ready!");  
      Serial.println();
      return;
    }
    delay(100);
  }
  Serial.println("Not ready... :(");  
  Serial.println();
}

void read_wafer_id() {
  Serial.println("Reading wafer id...");
  print_exchange_buffer(BS_READ_WAFER_ID, 0, sizeof(BS_READ_WAFER_ID) / sizeof(BS_READ_WAFER_ID[0]));
  Serial.println();
}

void hspi_send_command() {
  hspi->beginTransaction(SPISettings(spiClk, MSBFIRST, SPI_MODE));
  digitalWrite(HSPI_SS, LOW);
  //hspi->transferBytes(consuint8_t*data, uint8_t *out, uint32_t size)
  //hspi->writeBytes((uint8_t *)d, N);
  int tx = 0x8100;
  uint16_t rx = hspi->transfer16(tx);
  digitalWrite(HSPI_SS, HIGH);
  hspi->endTransaction();

  Serial.printf("tx/rx %04X/%04X", tx, rx);
  Serial.println();
}

int getState(int pin) {
  gpio_num_t pin0 = (gpio_num_t)(pin & 0x1F);
  int state = 0;
  if (GPIO_REG_READ(GPIO_ENABLE_REG) & BIT(pin0)) {
    //pin is output - read the GPIO_OUT_REG register
    state = (GPIO_REG_READ(GPIO_OUT_REG) >> pin0) & 1U;
  } else {
    //pin is input - read the GPIO_IN_REG register
    state = (GPIO_REG_READ(GPIO_IN_REG) >> pin0) & 1U;
  }
  return state;
}