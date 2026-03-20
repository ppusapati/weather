#!/usr/bin/env python3
"""Generate manufacturing-ready PCB layout for Weather Station.
Part 1: Header, nets, and core components (STM32, power, sensors)."""

import os

OUT = "/home/user/weather/hardware/kicad/WeatherStation/WeatherStation_mfg.kicad_pcb"

# Read existing PCB for header through line 100 (setup section)
with open("/home/user/weather/hardware/kicad/WeatherStation/WeatherStation.kicad_pcb") as f:
    existing = f.read()

# Extract everything up to the setup closing bracket
lines = existing.split('\n')
header_lines = []
for i, line in enumerate(lines):
    header_lines.append(line)
    if line.strip() == ')' and i > 90 and i < 110:
        break

header = '\n'.join(header_lines)

# Update placement zones for STM32 architecture
header = header.replace('ESP32-S3-WROOM-1\\n(U1)', 'STM32F407VGT6\\n(U10)')
header = header.replace('ESP32-S3 Module zone (center-top)', 'STM32F407 MCU zone (center-top)')

# ---- Net definitions ----
nets = """
  ;; =====================================================================
  ;; NET DEFINITIONS
  ;; =====================================================================

  (net 0 "")
  (net 1 "GND")
  (net 2 "+3V3")
  (net 3 "VBUS_5V")
  (net 4 "VBAT")
  (net 5 "VSOLAR")
  (net 6 "VBAT_SIM")

  ;; SPI1 - LoRa (SX1276)
  (net 10 "SPI1_MOSI")
  (net 11 "SPI1_MISO")
  (net 12 "SPI1_SCK")
  (net 13 "LORA_CS")
  (net 14 "LORA_RST")
  (net 15 "LORA_DIO0")
  (net 16 "LORA_DIO1")
  (net 17 "LORA_ANT")

  ;; SPI2 - W5500 Ethernet + SD Card (shared bus)
  (net 20 "SPI2_MOSI")
  (net 21 "SPI2_MISO")
  (net 22 "SPI2_SCK")
  (net 23 "ETH_CS")
  (net 24 "ETH_RST")
  (net 25 "ETH_INT")
  (net 26 "SD_CS")
  (net 27 "SD_DET")

  ;; SPI3 - ATWINC1500 WiFi
  (net 30 "SPI3_MOSI")
  (net 31 "SPI3_MISO")
  (net 32 "SPI3_SCK")
  (net 33 "WIFI_CS")
  (net 34 "WIFI_RST")
  (net 35 "WIFI_IRQ")
  (net 36 "WIFI_EN")
  (net 37 "WIFI_ANT")

  ;; I2C1 - Sensors
  (net 40 "I2C1_SDA")
  (net 41 "I2C1_SCL")

  ;; USART1 - Debug UART
  (net 42 "UART1_TX")
  (net 43 "UART1_RX")

  ;; USART2 - RS485 (MAX3485)
  (net 44 "UART2_TX")
  (net 45 "UART2_RX")
  (net 46 "RS485_A")
  (net 47 "RS485_B")
  (net 48 "RS485_DE")

  ;; USART3 - RN4870 BLE
  (net 50 "BLE_TX")
  (net 51 "BLE_RX")
  (net 52 "BLE_RST")
  (net 53 "BLE_STATUS")

  ;; UART4 - SIM7600 Cellular
  (net 54 "CELL_TX")
  (net 55 "CELL_RX")
  (net 56 "CELL_PWRKEY")
  (net 57 "CELL_STATUS")
  (net 58 "CELL_DTR")
  (net 59 "CELL_RI")

  ;; W5500 Ethernet PHY
  (net 60 "ETH_TXP")
  (net 61 "ETH_TXN")
  (net 62 "ETH_RXP")
  (net 63 "ETH_RXN")
  (net 64 "ETH_XTAL1")
  (net 65 "ETH_XTAL2")
  (net 66 "ETH_LINKLED")
  (net 67 "ETH_ACTLED")

  ;; SIM7600 SIM card
  (net 70 "SIM_VCC")
  (net 71 "SIM_RST")
  (net 72 "SIM_CLK")
  (net 73 "SIM_DATA")
  (net 74 "LTE_ANT")

  ;; STM32 crystal
  (net 75 "HSE_IN")
  (net 76 "HSE_OUT")
  (net 77 "NRST")
  (net 78 "BOOT0")

  ;; GPIO / misc
  (net 80 "USB_DP")
  (net 81 "USB_DM")
  (net 82 "CHRG_STAT")
  (net 83 "VBAT_SENSE")
  (net 84 "ANEM_SIG")
  (net 85 "RAIN_SIG")
  (net 86 "SOIL1_SIG")
  (net 87 "SOIL2_SIG")
  (net 88 "SOIL_TEMP_DATA")
  (net 89 "LEAF_WET_SIG")
  (net 90 "PYRANO_OUT")
  (net 91 "PANEL_TEMP1_DATA")
  (net 92 "PANEL_TEMP2_DATA")
  (net 93 "POWER_PULSE")
  (net 94 "MODE_SW1")
  (net 95 "MODE_SW2")
  (net 96 "STATUS_LED")
"""

# Write header + nets
with open(OUT, 'w') as f:
    f.write(header + '\n')
    f.write(nets)

print(f"Part 1 written: header + {96} nets")
