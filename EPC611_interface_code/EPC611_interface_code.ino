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
  0x42FF, // Integration length, high byte
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

/*
1M makes it
8M makes it
10M kiiinda makes it back
16M makes it there, but the return is garbled
*/
static const int spiClk = 1000000;

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

  Serial.println("Delaying 5 seconds");
  delay(5000);
  Serial.println("Done delaying");

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
}

// the loop function runs over and over again until power down or reset
void loop() {
  read_wafer_id();
  int dataRdy = digitalRead(DATA_RDY);
  Serial.printf("dataRdy: %d\n", dataRdy);
  delay(1000);
}

byte count = 0;

int print_exchange(int tx) {
  hspi->beginTransaction(SPISettings(spiClk, MSBFIRST, SPI_MODE));
  digitalWrite(HSPI_SS, LOW);
  uint16_t rx = hspi->transfer16(tx);
  digitalWrite(HSPI_SS, HIGH);
  hspi->endTransaction();
  Serial.printf("tx/rx %4X/%4X\n", tx, rx);
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
    Serial.printf("tx/rx %4X/%4X\n", tx[offset+i], exchangeBuffer[i]);
  }
}

void wait_ready() {
  //DUMMY Actually look at response
  Serial.println("delaying for ready....");
  for (int i = 0; i < 30; i++) {
    print_exchange(0x0000);
    delay(100);
  }
  Serial.println();
}

void read_wafer_id() {
  Serial.println("Reading wafer id...");
  // print_exchange(0x8700);
  // print_exchange(0x3600);
  // print_exchange(0x3700);
  // print_exchange(0x3800);
  // print_exchange(0x3900);
  // print_exchange(0x0000);
  print_exchange_buffer_slow(BS_READ_WAFER_ID, 0, sizeof(BS_READ_WAFER_ID) / sizeof(BS_READ_WAFER_ID[0]));
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

  Serial.printf("tx/rx %4X/%4X", tx, rx);
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