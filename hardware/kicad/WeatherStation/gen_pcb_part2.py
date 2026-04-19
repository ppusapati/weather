#!/usr/bin/env python3
"""Part 2: Component footprints - MCU, Power, Sensors."""

OUT = "/home/user/weather/hardware/kicad/WeatherStation/WeatherStation_mfg.kicad_pcb"

def smd_pad(num, x, y, w, h, net_id, net_name, layers='"F.Cu" "F.Paste" "F.Mask"'):
    return f'    (pad "{num}" smd rect (at {x} {y}) (size {w} {h}) (layers {layers}) (net {net_id} "{net_name}"))\n'

def smd_pad_r(num, x, y, w, h, net_id, net_name, rot=0):
    if rot:
        return f'    (pad "{num}" smd rect (at {x} {y} {rot}) (size {w} {h}) (layers "F.Cu" "F.Paste" "F.Mask") (net {net_id} "{net_name}"))\n'
    return smd_pad(num, x, y, w, h, net_id, net_name)

def thru_pad(num, x, y, pad_size, drill, net_id, net_name):
    return f'    (pad "{num}" thru_hole circle (at {x} {y}) (size {pad_size} {pad_size}) (drill {drill}) (layers "*.Cu" "*.Mask") (net {net_id} "{net_name}"))\n'

def fp_header(ref, value, footprint, x, y, rot=0):
    rot_str = f" {rot}" if rot else ""
    return f"""
  (footprint "{footprint}" (layer "F.Cu")
    (tstamp "fp-{ref.lower().replace(' ','')}-0001")
    (at {x} {y}{rot_str})
    (property "Reference" "{ref}" (at 0 -2) (layer "F.SilkS") (effects (font (size 1 1) (thickness 0.15))))
    (property "Value" "{value}" (at 0 2) (layer "F.Fab") (effects (font (size 1 1) (thickness 0.15))))
"""

def fp_end():
    return "  )\n"

def r0402(ref, value, x, y, rot, net1_id, net1_name, net2_id, net2_name):
    """0402 resistor (1.0x0.5mm pads)"""
    s = fp_header(ref, value, "Resistor_SMD:R_0402_1005Metric", x, y, rot)
    s += smd_pad("1", -0.48, 0, 0.56, 0.62, net1_id, net1_name)
    s += smd_pad("2", 0.48, 0, 0.56, 0.62, net2_id, net2_name)
    s += fp_end()
    return s

def c0402(ref, value, x, y, rot, net1_id, net1_name, net2_id, net2_name):
    """0402 capacitor"""
    s = fp_header(ref, value, "Capacitor_SMD:C_0402_1005Metric", x, y, rot)
    s += smd_pad("1", -0.48, 0, 0.56, 0.62, net1_id, net1_name)
    s += smd_pad("2", 0.48, 0, 0.56, 0.62, net2_id, net2_name)
    s += fp_end()
    return s

def c0805(ref, value, x, y, rot, net1_id, net1_name, net2_id, net2_name):
    """0805 capacitor"""
    s = fp_header(ref, value, "Capacitor_SMD:C_0805_2012Metric", x, y, rot)
    s += smd_pad("1", -0.95, 0, 1.0, 1.45, net1_id, net1_name)
    s += smd_pad("2", 0.95, 0, 1.0, 1.45, net2_id, net2_name)
    s += fp_end()
    return s

def led_0603(ref, value, x, y, rot, anode_net_id, anode_net, cath_net_id, cath_net):
    """0603 LED"""
    s = fp_header(ref, value, "LED_SMD:LED_0603_1608Metric", x, y, rot)
    s += smd_pad("1", -0.75, 0, 0.8, 0.8, cath_net_id, cath_net)
    s += smd_pad("2", 0.75, 0, 0.8, 0.8, anode_net_id, anode_net)
    s += fp_end()
    return s

components = ""

# =====================================================================
# U10: STM32F407VGT6 LQFP-100 (14x14mm, 0.5mm pitch)
# Placed at center-top: (50, 22)
# =====================================================================
components += """
  ;; =====================================================================
  ;; U10: STM32F407VGT6 — LQFP-100, center of board
  ;; =====================================================================
"""
components += fp_header("U10", "STM32F407VGT6", "Package_QFP:LQFP-100_14x14mm_P0.5mm", 50, 22)

# LQFP-100: 25 pins per side, 0.5mm pitch
# Pin 1 at top-left, going counter-clockwise
# Side 1 (left): pins 1-25, x=-7.75, y from -6.0 to +6.0
# Side 2 (bottom): pins 26-50, y=7.75, x from -6.0 to +6.0
# Side 3 (right): pins 51-75, x=7.75, y from 6.0 to -6.0
# Side 4 (top): pins 76-100, y=-7.75, x from 6.0 to -6.0

# Key pin-to-net assignments for STM32F407VGT6:
stm32_pin_nets = {
    # Left side (pins 1-25): PE2-PE6, PB3-PB5, etc.
    1: (59, "CELL_RI"),       # PE2
    2: (36, "WIFI_EN"),       # PE3 -> WIFI_CS actually. Let me fix.
    # Actually PE3=WIFI_CS, PE4=WIFI_RST, PE5=WIFI_IRQ, PE6=WIFI_EN
    3: (33, "WIFI_CS"),       # PE3
    4: (34, "WIFI_RST"),      # PE4
    5: (35, "WIFI_IRQ"),      # PE5
    6: (36, "WIFI_EN"),       # PE6
    7: (2, "+3V3"),           # VBAT
    23: (10, "SPI1_MOSI"),    # PA7 (SPI1_MOSI)
    24: (12, "SPI1_SCK"),     # PA5 (SPI1_SCK)
    25: (11, "SPI1_MISO"),    # PA6 (SPI1_MISO)

    # Bottom side (pins 26-50)
    26: (40, "I2C1_SDA"),     # PB7 (I2C1_SDA)
    27: (41, "I2C1_SCL"),     # PB6 (I2C1_SCL)
    29: (50, "BLE_TX"),       # PB10 (USART3_TX)
    30: (51, "BLE_RX"),       # PB11 (USART3_RX)
    31: (23, "ETH_CS"),       # PB12 (SPI2_NSS)
    32: (22, "SPI2_SCK"),     # PB13 (SPI2_SCK)
    33: (21, "SPI2_MISO"),    # PB14 (SPI2_MISO)
    34: (20, "SPI2_MOSI"),    # PB15 (SPI2_MOSI)
    47: (2, "+3V3"),          # VDD
    48: (1, "GND"),           # VSS

    # Right side (pins 51-75)
    51: (32, "SPI3_SCK"),     # PB3 (SPI3_SCK)
    52: (31, "SPI3_MISO"),    # PB4 (SPI3_MISO)
    53: (30, "SPI3_MOSI"),    # PB5 (SPI3_MOSI)
    58: (24, "ETH_RST"),      # PD3
    59: (25, "ETH_INT"),      # PD4
    60: (56, "CELL_PWRKEY"),  # PD5
    61: (57, "CELL_STATUS"),  # PD6
    62: (58, "CELL_DTR"),     # PD7
    63: (52, "BLE_RST"),      # PD8
    64: (53, "BLE_STATUS"),   # PD9
    65: (26, "SD_CS"),        # PD10
    66: (27, "SD_DET"),       # PD11
    72: (2, "+3V3"),          # VDD
    73: (1, "GND"),           # VSS

    # Top side (pins 76-100)
    78: (54, "CELL_TX"),      # PC10 (UART4_TX)
    79: (55, "CELL_RX"),      # PC11 (UART4_RX)
    81: (44, "UART2_TX"),     # PA2 (USART2_TX for RS485)
    82: (45, "UART2_RX"),     # PA3 (USART2_RX for RS485)
    83: (13, "LORA_CS"),      # PA4 (SPI1_NSS)
    87: (80, "USB_DP"),       # PA11 (USB_DP)
    88: (81, "USB_DM"),       # PA12 (USB_DM)
    90: (75, "HSE_IN"),       # PH0/OSC_IN
    91: (76, "HSE_OUT"),      # PH1/OSC_OUT
    93: (77, "NRST"),         # NRST
    94: (14, "LORA_RST"),     # PC4
    95: (15, "LORA_DIO0"),    # PC5
    99: (2, "+3V3"),          # VDD
    100: (1, "GND"),          # VSS
}

# Generate all 100 pads
for side in range(4):
    for pin_idx in range(25):
        pin_num = side * 25 + pin_idx + 1
        if side == 0:  # Left side
            px, py = -7.75, -6.0 + pin_idx * 0.5
        elif side == 1:  # Bottom side
            px, py = -6.0 + pin_idx * 0.5, 7.75
        elif side == 2:  # Right side
            px, py = 7.75, 6.0 - pin_idx * 0.5
        else:  # Top side
            px, py = 6.0 - pin_idx * 0.5, -7.75

        net_id, net_name = stm32_pin_nets.get(pin_num, (0, ""))

        if side in (0, 2):  # Left/Right: vertical pads
            components += f'    (pad "{pin_num}" smd rect (at {px} {py}) (size 1.5 0.3) (layers "F.Cu" "F.Paste" "F.Mask") (net {net_id} "{net_name}"))\n'
        else:  # Top/Bottom: horizontal pads
            components += f'    (pad "{pin_num}" smd rect (at {px} {py}) (size 0.3 1.5) (layers "F.Cu" "F.Paste" "F.Mask") (net {net_id} "{net_name}"))\n'

# Exposed pad (GND)
components += f'    (pad "101" smd rect (at 0 0) (size 7.7 7.7) (layers "F.Cu" "F.Paste" "F.Mask") (net 1 "GND"))\n'
components += fp_end()

# =====================================================================
# Y1: 8MHz Crystal for STM32
# =====================================================================
components += fp_header("Y1", "8MHz", "Crystal:Crystal_SMD_HC49-SD", 42, 14)
components += smd_pad("1", -2.4, 0, 1.5, 2.4, 75, "HSE_IN")
components += smd_pad("2", 2.4, 0, 1.5, 2.4, 76, "HSE_OUT")
components += fp_end()

# C20, C21: 20pF crystal load caps
components += c0402("C20", "20pF", 40, 16, 0, 75, "HSE_IN", 1, "GND")
components += c0402("C21", "20pF", 44, 16, 0, 76, "HSE_OUT", 1, "GND")

# STM32 decoupling: C22a-d (100nF x4), C23 (4.7uF), C24 (NRST 100nF)
components += c0402("C22a", "100nF", 44, 18, 90, 2, "+3V3", 1, "GND")
components += c0402("C22b", "100nF", 46, 18, 90, 2, "+3V3", 1, "GND")
components += c0402("C22c", "100nF", 54, 18, 90, 2, "+3V3", 1, "GND")
components += c0402("C22d", "100nF", 56, 18, 90, 2, "+3V3", 1, "GND")
components += c0805("C23", "4.7uF", 58, 22, 90, 2, "+3V3", 1, "GND")
components += c0402("C24", "100nF", 40, 12, 0, 77, "NRST", 1, "GND")

# R20 (NRST pull-up), R21 (BOOT0 pull-down)
components += r0402("R20", "10k", 38, 14, 90, 2, "+3V3", 77, "NRST")
components += r0402("R21", "10k", 60, 14, 90, 78, "BOOT0", 1, "GND")

# SW3: Mode DIP switch
components += fp_header("SW3", "MODE_DIP", "Switch_SMD:SW_DIP_SPSTx02_Slide_6.7x6.64mm_W8.61mm_P2.54mm", 62, 14)
components += smd_pad("1", -3.81, -1.27, 1.5, 1.0, 94, "MODE_SW1")
components += smd_pad("2", -3.81, 1.27, 1.5, 1.0, 95, "MODE_SW2")
components += smd_pad("3", 3.81, 1.27, 1.5, 1.0, 1, "GND")
components += smd_pad("4", 3.81, -1.27, 1.5, 1.0, 1, "GND")
components += fp_end()

# R25-R26: mode switch pull-downs
components += r0402("R25", "10k", 62, 9, 0, 94, "MODE_SW1", 1, "GND")
components += r0402("R26", "10k", 62, 11, 0, 95, "MODE_SW2", 1, "GND")

# =====================================================================
# U11: MAX3485 RS485 — SOIC-8, near STM32
# =====================================================================
components += fp_header("U11", "MAX3485ESA+", "Package_SO:SOIC-8_3.9x4.9mm_P1.27mm", 68, 22)
for i, (ny, nid, nn) in enumerate([
    (-1.905, 45, "UART2_RX"),     # 1: RO
    (-0.635, 48, "RS485_DE"),     # 2: RE (tied to DE)
    (0.635, 48, "RS485_DE"),      # 3: DE
    (1.905, 44, "UART2_TX"),      # 4: DI
]):
    components += smd_pad(str(i+1), -2.7, ny, 1.5, 0.6, nid, nn)
for i, (ny, nid, nn) in enumerate([
    (1.905, 1, "GND"),            # 5: GND
    (0.635, 46, "RS485_A"),       # 6: A
    (-0.635, 47, "RS485_B"),      # 7: B
    (-1.905, 2, "+3V3"),          # 8: VCC
]):
    components += smd_pad(str(i+5), 2.7, ny, 1.5, 0.6, nid, nn)
components += fp_end()

# C25: MAX3485 decoupling
components += c0402("C25", "100nF", 68, 17, 0, 2, "+3V3", 1, "GND")

# R22: RS485 termination
components += r0402("R22", "120R", 72, 22, 90, 46, "RS485_A", 47, "RS485_B")

# J16: RS485 Phoenix Contact 4-pin terminal
components += fp_header("J16", "RS485_Bus", "Connector_Phoenix_MC:PhoenixContact_MC_01x04_G_3.5mm", 78, 22)
components += thru_pad("1", -5.25, 0, 2.0, 1.1, 46, "RS485_A")
components += thru_pad("2", -1.75, 0, 2.0, 1.1, 47, "RS485_B")
components += thru_pad("3", 1.75, 0, 2.0, 1.1, 1, "GND")
components += thru_pad("4", 5.25, 0, 2.0, 1.1, 1, "GND")
components += fp_end()

# D10: RS485 TVS diode
components += fp_header("D10", "SMBJ6.0CA", "Diode_SMD:D_SMB", 74, 18)
components += smd_pad("1", -2.1, 0, 2.0, 2.3, 46, "RS485_A")
components += smd_pad("2", 2.1, 0, 2.0, 2.3, 47, "RS485_B")
components += fp_end()

# =====================================================================
# POWER SUPPLY SECTION (bottom-left: x=5-30, y=50-75)
# =====================================================================
components += """
  ;; =====================================================================
  ;; POWER SUPPLY: TP4056, DW01A, FS8205A, AMS1117
  ;; =====================================================================
"""

# J1: USB-C connector (bottom edge, center)
components += fp_header("J1", "USB-C", "Connector_USB:USB_C_Receptacle_GCT_USB4105", 36, 78, 180)
components += smd_pad("A1", -3.25, -4.32, 0.3, 1.0, 1, "GND")
components += smd_pad("A4", -2.45, -4.32, 0.3, 1.0, 3, "VBUS_5V")
components += smd_pad("A5", -0.25, -4.32, 0.3, 1.0, 0, "")
components += smd_pad("A6", 0.25, -4.32, 0.3, 1.0, 80, "USB_DP")
components += smd_pad("A7", 0.75, -4.32, 0.3, 1.0, 81, "USB_DM")
components += smd_pad("A8", 1.25, -4.32, 0.3, 1.0, 0, "")
components += smd_pad("A9", 2.45, -4.32, 0.3, 1.0, 3, "VBUS_5V")
components += smd_pad("A12", 3.25, -4.32, 0.3, 1.0, 1, "GND")
components += smd_pad("B1", 3.25, 4.32, 0.3, 1.0, 1, "GND")
components += smd_pad("B4", 2.45, 4.32, 0.3, 1.0, 3, "VBUS_5V")
components += smd_pad("B9", -2.45, 4.32, 0.3, 1.0, 3, "VBUS_5V")
components += smd_pad("B12", -3.25, 4.32, 0.3, 1.0, 1, "GND")
components += smd_pad("S1", -4.32, 0, 0.6, 1.2, 1, "GND")
components += smd_pad("S2", 4.32, 0, 0.6, 1.2, 1, "GND")
components += fp_end()

# R1a, R1b: USB-C CC pulldowns (5.1k)
components += r0402("R1a", "5.1k", 32, 74, 0, 1, "GND", 0, "")
components += r0402("R1b", "5.1k", 40, 74, 0, 1, "GND", 0, "")

# F1: Polyfuse
components += fp_header("F1", "500mA", "Fuse_SMD:Fuse_1206_3216Metric", 30, 72)
components += smd_pad("1", -1.5, 0, 1.2, 1.8, 3, "VBUS_5V")
components += smd_pad("2", 1.5, 0, 1.2, 1.8, 3, "VBUS_5V")
components += fp_end()

# D2: USB TVS
components += fp_header("D2", "SMAJ5.0A", "Diode_SMD:D_SMA", 34, 72)
components += smd_pad("1", -2.1, 0, 1.4, 1.8, 3, "VBUS_5V")
components += smd_pad("2", 2.1, 0, 1.4, 1.8, 1, "GND")
components += fp_end()

# U2: TP4056 SOIC-8 (battery charger)
components += fp_header("U2", "TP4056", "Package_SO:SOIC-8_3.9x4.9mm_P1.27mm", 12, 58)
for i, (ny, nid, nn) in enumerate([
    (-1.905, 82, "CHRG_STAT"),  # 1: CHRG
    (-0.635, 0, ""),            # 2: STDBY
    (0.635, 3, "VBUS_5V"),     # 3: VCC
    (1.905, 4, "VBAT"),        # 4: BAT
]):
    components += smd_pad(str(i+1), -2.7, ny, 1.5, 0.6, nid, nn)
for i, (ny, nid, nn) in enumerate([
    (1.905, 1, "GND"),         # 5: TEMP
    (-0.635, 1, "GND"),        # 7: GND
    (-1.905, 0, ""),           # 8: RPROG
]):
    idx = i + 5
    if idx == 5:
        components += smd_pad("5", 2.7, 1.905, 1.5, 0.6, 1, "GND")
    elif idx == 6:
        components += smd_pad("6", 2.7, 0.635, 1.5, 0.6, 0, "")
    elif idx == 7:
        components += smd_pad("7", 2.7, -0.635, 1.5, 0.6, 1, "GND")
components += smd_pad("8", 2.7, -1.905, 1.5, 0.6, 0, "")
components += fp_end()

# R2: RPROG for TP4056
components += r0402("R2", "2.2k", 16, 56, 90, 0, "", 1, "GND")

# J2: Solar input connector
components += fp_header("J2", "SOLAR_IN", "Connector_JST:JST_PH_B2B-PH-K_1x02_P2.00mm_Vertical", 5, 68)
components += thru_pad("1", -1.0, 0, 1.7, 0.8, 5, "VSOLAR")
components += thru_pad("2", 1.0, 0, 1.7, 0.8, 1, "GND")
components += fp_end()

# D1: Schottky diode for solar
components += fp_header("D1", "SS14", "Diode_SMD:D_SMA", 10, 64)
components += smd_pad("1", -2.1, 0, 1.4, 1.8, 5, "VSOLAR")
components += smd_pad("2", 2.1, 0, 1.4, 1.8, 3, "VBUS_5V")
components += fp_end()

# J3: Battery connector
components += fp_header("J3", "BATTERY", "Connector_JST:JST_PH_B2B-PH-K_1x02_P2.00mm_Vertical", 5, 56)
components += thru_pad("1", -1.0, 0, 1.7, 0.8, 4, "VBAT")
components += thru_pad("2", 1.0, 0, 1.7, 0.8, 1, "GND")
components += fp_end()

# U3: AMS1117-3.3 SOT-223
components += fp_header("U3", "AMS1117-3.3", "Package_TO_SOT_SMD:SOT-223-3_TabPin2", 22, 66)
components += smd_pad("1", -2.3, 0, 1.2, 0.7, 1, "GND")
components += smd_pad("2", 0, 3.15, 3.0, 2.0, 2, "+3V3")
components += smd_pad("3", 2.3, 0, 1.2, 0.7, 4, "VBAT")
components += fp_end()

# C3: AMS1117 output 22uF
components += c0805("C3", "22uF", 28, 66, 0, 2, "+3V3", 1, "GND")
# C5: TP4056 input 10uF
components += c0805("C5", "10uF", 18, 62, 0, 3, "VBUS_5V", 1, "GND")
# C6: AMS1117 input 10uF
components += c0805("C6", "10uF", 28, 62, 0, 4, "VBAT", 1, "GND")

# LEDs: D3 (red charging), D4 (green standby), D5 (blue power)
components += led_0603("D3", "LED_Red", 20, 54, 0, 82, "CHRG_STAT", 1, "GND")
components += led_0603("D4", "LED_Green", 23, 54, 0, 0, "", 1, "GND")
components += led_0603("D5", "LED_Blue", 26, 54, 0, 2, "+3V3", 1, "GND")
# R6a-c: LED series resistors
components += r0402("R6a", "1k", 20, 52, 90, 82, "CHRG_STAT", 0, "")
components += r0402("R6b", "1k", 23, 52, 90, 0, "", 0, "")
components += r0402("R6c", "1k", 26, 52, 90, 2, "+3V3", 0, "")

# R7a/R7b: Battery voltage divider
components += r0402("R7a", "100k", 30, 56, 90, 4, "VBAT", 83, "VBAT_SENSE")
components += r0402("R7b", "100k", 30, 58, 90, 83, "VBAT_SENSE", 1, "GND")

# SW1: BOOT, SW2: RESET
components += fp_header("SW1", "BOOT", "Button_SMD:SW_SPST_PTS645", 30, 72)
components += smd_pad("1", -3.25, 0, 1.5, 1.0, 78, "BOOT0")
components += smd_pad("2", 3.25, 0, 1.5, 1.0, 1, "GND")
components += fp_end()

components += fp_header("SW2", "RESET", "Button_SMD:SW_SPST_PTS645", 24, 72)
components += smd_pad("1", -3.25, 0, 1.5, 1.0, 77, "NRST")
components += smd_pad("2", 3.25, 0, 1.5, 1.0, 1, "GND")
components += fp_end()

# =====================================================================
# I2C SENSORS (right side: x=80-96, y=12-45)
# =====================================================================
components += """
  ;; =====================================================================
  ;; I2C SENSORS: BME280, AS5600, SI1145, BH1750
  ;; =====================================================================
"""

# U4: BME280 LGA-8 (2.5x2.5mm)
components += fp_header("U4", "BME280", "Package_LGA:Bosch_BME280_LGA-8_2.5x2.5mm", 84, 16)
components += smd_pad("1", -0.625, -0.6, 0.5, 0.5, 1, "GND")
components += smd_pad("2", 0.625, -0.6, 0.5, 0.5, 0, "")
components += smd_pad("3", -0.625, 0, 0.5, 0.5, 1, "GND")
components += smd_pad("4", 0.625, 0, 0.5, 0.5, 0, "")
components += smd_pad("5", -0.625, 0.6, 0.5, 0.5, 2, "+3V3")
components += smd_pad("6", 0.625, 0.6, 0.5, 0.5, 40, "I2C1_SDA")
components += smd_pad("7", 0, -0.6, 0.5, 0.5, 41, "I2C1_SCL")
components += smd_pad("8", 0, 0.6, 0.5, 0.5, 2, "+3V3")
components += fp_end()

# U5: AS5600 SOIC-8
components += fp_header("U5", "AS5600", "Package_SO:SOIC-8_3.9x4.9mm_P1.27mm", 90, 16)
for i, (ny, nid, nn) in enumerate([
    (-1.905, 40, "I2C1_SDA"),   # SDA
    (-0.635, 41, "I2C1_SCL"),   # SCL
    (0.635, 1, "GND"),          # GND
    (1.905, 0, ""),             # DIR
]):
    components += smd_pad(str(i+1), -2.7, ny, 1.5, 0.6, nid, nn)
for i, (ny, nid, nn) in enumerate([
    (1.905, 0, ""),  # OUT
    (0.635, 0, ""),  # PGO
    (-0.635, 2, "+3V3"),  # VDD5V
    (-1.905, 2, "+3V3"),  # VDD3V3
]):
    components += smd_pad(str(i+5), 2.7, ny, 1.5, 0.6, nid, nn)
components += fp_end()

# U6: SI1145 QFN-10 (3x3mm)
components += fp_header("U6", "SI1145", "Package_DFN_QFN:QFN-10-1EP_3x3mm_P0.6mm", 84, 24)
components += smd_pad("1", -1.45, -0.6, 0.8, 0.35, 40, "I2C1_SDA")
components += smd_pad("2", -1.45, 0, 0.8, 0.35, 41, "I2C1_SCL")
components += smd_pad("3", -1.45, 0.6, 0.8, 0.35, 0, "")
components += smd_pad("6", 1.45, 0.6, 0.8, 0.35, 2, "+3V3")
components += smd_pad("7", 1.45, 0, 0.8, 0.35, 1, "GND")
components += smd_pad("8", 1.45, -0.6, 0.8, 0.35, 0, "")
components += smd_pad("11", 0, 0, 1.7, 1.7, 1, "GND")  # EP
components += fp_end()

# U7: BH1750 SOIC-8
components += fp_header("U7", "BH1750FVI", "Package_SO:SOIC-8_3.9x4.9mm_P1.27mm", 90, 24)
for i, (ny, nid, nn) in enumerate([
    (-1.905, 2, "+3V3"),   # VCC
    (-0.635, 1, "GND"),   # ADDR
    (0.635, 1, "GND"),    # GND
    (1.905, 40, "I2C1_SDA"),  # SDA
]):
    components += smd_pad(str(i+1), -2.7, ny, 1.5, 0.6, nid, nn)
for i, (ny, nid, nn) in enumerate([
    (1.905, 0, ""),        # DVI
    (0.635, 41, "I2C1_SCL"),  # SCL
    (-0.635, 0, ""),       # NC
    (-1.905, 2, "+3V3"),   # VCC
]):
    components += smd_pad(str(i+5), 2.7, ny, 1.5, 0.6, nid, nn)
components += fp_end()

# I2C pull-ups
components += r0402("R8a", "4.7k", 82, 20, 90, 2, "+3V3", 40, "I2C1_SDA")
components += r0402("R8b", "4.7k", 82, 22, 90, 2, "+3V3", 41, "I2C1_SCL")

# Sensor decoupling caps
components += c0402("C_U4", "100nF", 86, 14, 0, 2, "+3V3", 1, "GND")
components += c0402("C_U5", "100nF", 94, 14, 0, 2, "+3V3", 1, "GND")
components += c0402("C_U6", "100nF", 86, 26, 0, 2, "+3V3", 1, "GND")
components += c0402("C_U7", "100nF", 94, 26, 0, 2, "+3V3", 1, "GND")

# Append to file
with open(OUT, 'a') as f:
    f.write(components)

print("Part 2 written: STM32, power supply, sensors, RS485")
