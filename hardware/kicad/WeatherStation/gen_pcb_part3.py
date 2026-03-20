#!/usr/bin/env python3
"""Part 3: Communication modules (LoRa, WiFi, BLE, Ethernet, Cellular) + Connectors."""

OUT = "/home/user/weather/hardware/kicad/WeatherStation/WeatherStation_mfg.kicad_pcb"

def smd_pad(num, x, y, w, h, net_id, net_name):
    return f'    (pad "{num}" smd rect (at {x} {y}) (size {w} {h}) (layers "F.Cu" "F.Paste" "F.Mask") (net {net_id} "{net_name}"))\n'

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

def r0402(ref, value, x, y, rot, n1id, n1, n2id, n2):
    s = fp_header(ref, value, "Resistor_SMD:R_0402_1005Metric", x, y, rot)
    s += smd_pad("1", -0.48, 0, 0.56, 0.62, n1id, n1)
    s += smd_pad("2", 0.48, 0, 0.56, 0.62, n2id, n2)
    s += fp_end()
    return s

def c0402(ref, value, x, y, rot, n1id, n1, n2id, n2):
    s = fp_header(ref, value, "Capacitor_SMD:C_0402_1005Metric", x, y, rot)
    s += smd_pad("1", -0.48, 0, 0.56, 0.62, n1id, n1)
    s += smd_pad("2", 0.48, 0, 0.56, 0.62, n2id, n2)
    s += fp_end()
    return s

def c0805(ref, value, x, y, rot, n1id, n1, n2id, n2):
    s = fp_header(ref, value, "Capacitor_SMD:C_0805_2012Metric", x, y, rot)
    s += smd_pad("1", -0.95, 0, 1.0, 1.45, n1id, n1)
    s += smd_pad("2", 0.95, 0, 1.0, 1.45, n2id, n2)
    s += fp_end()
    return s

components = ""

# =====================================================================
# U8: SX1276/RFM95W LoRa Module (left side: x=5-20, y=14-30)
# =====================================================================
components += """
  ;; =====================================================================
  ;; U8: SX1276 LoRa + impedance matching + SMA
  ;; =====================================================================
"""
components += fp_header("U8", "SX1276/RFM95W", "Module:RFM95W", 12, 30)
# RFM95W module has castellated pads
for i, (px, py, nid, nn) in enumerate([
    (-8, -5, 1, "GND"),      # 1: GND
    (-8, -3, 10, "SPI1_MOSI"),  # 2: MOSI
    (-8, -1, 11, "SPI1_MISO"),  # 3: MISO
    (-8, 1, 12, "SPI1_SCK"),    # 4: SCK
    (-8, 3, 13, "LORA_CS"),     # 5: NSS
    (-8, 5, 14, "LORA_RST"),    # 6: RESET
    (8, 5, 15, "LORA_DIO0"),    # 7: DIO0
    (8, 3, 16, "LORA_DIO1"),    # 8: DIO1
    (8, 1, 0, ""),              # 9: DIO2
    (8, -1, 0, ""),             # 10: DIO3
    (8, -3, 0, ""),             # 11: DIO4
    (8, -5, 17, "LORA_ANT"),   # 12: ANT
    (0, 7, 1, "GND"),          # 13: GND
    (0, -7, 2, "+3V3"),        # 14: VCC
]):
    components += smd_pad(str(i+1), px, py, 1.5, 1.0, nid, nn)
components += smd_pad("15", 0, 0, 5, 5, 1, "GND")  # GND pad
components += fp_end()

# LoRa impedance matching network
components += fp_header("L1", "5.6nH", "Inductor_SMD:L_0402_1005Metric", 12, 38)
components += smd_pad("1", -0.48, 0, 0.56, 0.62, 17, "LORA_ANT")
components += smd_pad("2", 0.48, 0, 0.56, 0.62, 0, "")
components += fp_end()

components += c0402("C12", "1.5pF", 10, 40, 0, 17, "LORA_ANT", 1, "GND")
components += c0402("C13", "3.3pF", 14, 40, 0, 0, "", 1, "GND")

# J6: LoRa SMA (left edge)
components += fp_header("J6", "SMA_LoRa", "Connector_Coaxial:SMA_Amphenol_132289_EdgeMount", 0, 38, 90)
components += smd_pad("1", 0, 0, 1.5, 1.5, 0, "")
components += smd_pad("2", -2.5, 0, 1.5, 1.5, 1, "GND")
components += smd_pad("3", 2.5, 0, 1.5, 1.5, 1, "GND")
components += fp_end()

# C10, C11: LoRa decoupling
components += c0805("C10", "10uF", 12, 22, 0, 2, "+3V3", 1, "GND")
components += c0402("C11", "100nF", 12, 24, 0, 2, "+3V3", 1, "GND")

# =====================================================================
# U12: ATWINC1500 WiFi QFN-28 (near STM32, x=35, y=38)
# =====================================================================
components += """
  ;; =====================================================================
  ;; U12: ATWINC1500 WiFi + SMA + decoupling
  ;; =====================================================================
"""
components += fp_header("U12", "ATWINC1500", "Package_DFN_QFN:QFN-28-1EP_5x5mm_P0.5mm_EP3.1x3.1mm", 35, 38)
# QFN-28: 7 pins per side, 0.5mm pitch
# Simplified: key pins with nets
pin_nets_u12 = {
    1: (32, "SPI3_SCK"),
    2: (31, "SPI3_MISO"),
    3: (30, "SPI3_MOSI"),
    4: (33, "WIFI_CS"),
    5: (34, "WIFI_RST"),
    6: (35, "WIFI_IRQ"),
    7: (36, "WIFI_EN"),
    14: (2, "+3V3"),    # VCC
    21: (1, "GND"),     # GND
    28: (37, "WIFI_ANT"),  # ANT
}
for side in range(4):
    for pin_idx in range(7):
        pin_num = side * 7 + pin_idx + 1
        if side == 0:   # Left
            px, py = -2.85, -1.5 + pin_idx * 0.5
        elif side == 1: # Bottom
            px, py = -1.5 + pin_idx * 0.5, 2.85
        elif side == 2: # Right
            px, py = 2.85, 1.5 - pin_idx * 0.5
        else:           # Top
            px, py = 1.5 - pin_idx * 0.5, -2.85

        nid, nn = pin_nets_u12.get(pin_num, (0, ""))
        if side in (0, 2):
            components += smd_pad(str(pin_num), px, py, 0.8, 0.25, nid, nn)
        else:
            components += smd_pad(str(pin_num), px, py, 0.25, 0.8, nid, nn)
components += smd_pad("29", 0, 0, 3.1, 3.1, 1, "GND")  # EP
components += fp_end()

# ATWINC1500 decoupling
components += c0402("C28a", "100nF", 35, 33, 0, 2, "+3V3", 1, "GND")
components += c0805("C28b", "10uF", 35, 35, 0, 2, "+3V3", 1, "GND")
# R3: CHIP_EN pull-up
components += r0402("R3", "10k", 30, 38, 90, 2, "+3V3", 36, "WIFI_EN")

# J18: WiFi SMA (left edge, below LoRa)
components += fp_header("J18", "SMA_WiFi", "Connector_Coaxial:SMA_Amphenol_132289_EdgeMount", 0, 48, 90)
components += smd_pad("1", 0, 0, 1.5, 1.5, 37, "WIFI_ANT")
components += smd_pad("2", -2.5, 0, 1.5, 1.5, 1, "GND")
components += smd_pad("3", 2.5, 0, 1.5, 1.5, 1, "GND")
components += fp_end()

# =====================================================================
# U13: RN4870 BLE Module (near STM32, x=50, y=38)
# =====================================================================
components += """
  ;; =====================================================================
  ;; U13: RN4870 BLE Module + decoupling
  ;; =====================================================================
"""
components += fp_header("U13", "RN4870-I/RM", "WeatherStation:RN4870_11.5x8mm", 50, 40)
components += smd_pad("1", -4.75, -2, 1.5, 0.8, 50, "BLE_TX")
components += smd_pad("2", -4.75, 0, 1.5, 0.8, 51, "BLE_RX")
components += smd_pad("3", -4.75, 2, 1.5, 0.8, 52, "BLE_RST")
components += smd_pad("4", 4.75, 2, 1.5, 0.8, 53, "BLE_STATUS")
components += smd_pad("5", 4.75, 0, 1.5, 0.8, 2, "+3V3")
components += smd_pad("6", 4.75, -2, 1.5, 0.8, 1, "GND")
components += smd_pad("7", 0, 4, 3.0, 1.5, 1, "GND")  # GND pad
components += fp_end()

# BLE decoupling
components += c0402("C30", "100nF", 54, 36, 0, 2, "+3V3", 1, "GND")
components += c0805("C29", "4.7uF", 56, 38, 0, 2, "+3V3", 1, "GND")
# R4: BLE RST pull-up
components += r0402("R4", "10k", 46, 36, 0, 2, "+3V3", 52, "BLE_RST")
# R23-R24: UART series protection
components += r0402("R23", "100R", 46, 38, 0, 50, "BLE_TX", 0, "")
components += r0402("R24", "100R", 46, 40, 0, 51, "BLE_RX", 0, "")

# =====================================================================
# U14: W5500 Ethernet LQFP-48 (bottom center: x=60, y=55)
# =====================================================================
components += """
  ;; =====================================================================
  ;; U14: W5500 Ethernet + 25MHz crystal + RJ45
  ;; =====================================================================
"""
components += fp_header("U14", "W5500", "Package_QFP:LQFP-48_7x7mm_P0.5mm", 60, 55)
# LQFP-48: 12 pins per side, 0.5mm pitch
w5500_pin_nets = {
    1: (22, "SPI2_SCK"),
    2: (21, "SPI2_MISO"),
    3: (20, "SPI2_MOSI"),
    4: (23, "ETH_CS"),
    5: (24, "ETH_RST"),
    6: (25, "ETH_INT"),
    12: (2, "+3V3"),      # VCC
    24: (1, "GND"),       # GND
    25: (60, "ETH_TXP"),
    26: (61, "ETH_TXN"),
    27: (62, "ETH_RXP"),
    28: (63, "ETH_RXN"),
    35: (64, "ETH_XTAL1"),
    36: (65, "ETH_XTAL2"),
    47: (66, "ETH_LINKLED"),
    48: (67, "ETH_ACTLED"),
}
for side in range(4):
    for pin_idx in range(12):
        pin_num = side * 12 + pin_idx + 1
        if side == 0:   # Left
            px, py = -4.35, -2.75 + pin_idx * 0.5
        elif side == 1: # Bottom
            px, py = -2.75 + pin_idx * 0.5, 4.35
        elif side == 2: # Right
            px, py = 4.35, 2.75 - pin_idx * 0.5
        else:           # Top
            px, py = 2.75 - pin_idx * 0.5, -4.35
        nid, nn = w5500_pin_nets.get(pin_num, (0, ""))
        if side in (0, 2):
            components += smd_pad(str(pin_num), px, py, 1.2, 0.28, nid, nn)
        else:
            components += smd_pad(str(pin_num), px, py, 0.28, 1.2, nid, nn)
components += smd_pad("49", 0, 0, 3.5, 3.5, 1, "GND")  # EP
components += fp_end()

# Y2: 25MHz crystal
components += fp_header("Y2", "25MHz", "Crystal:Crystal_SMD_HC49-SD", 68, 52)
components += smd_pad("1", -2.4, 0, 1.5, 2.4, 64, "ETH_XTAL1")
components += smd_pad("2", 2.4, 0, 1.5, 2.4, 65, "ETH_XTAL2")
components += fp_end()

# C26-C27: W5500 crystal load caps
components += c0402("C26", "18pF", 66, 55, 90, 64, "ETH_XTAL1", 1, "GND")
components += c0402("C27", "18pF", 70, 55, 90, 65, "ETH_XTAL2", 1, "GND")

# W5500 decoupling
components += c0402("C_W5a", "100nF", 55, 50, 0, 2, "+3V3", 1, "GND")
components += c0402("C_W5b", "100nF", 57, 50, 0, 2, "+3V3", 1, "GND")
components += c0402("C_W5c", "100nF", 63, 50, 0, 2, "+3V3", 1, "GND")
components += c0402("C_W5d", "100nF", 65, 50, 0, 2, "+3V3", 1, "GND")
components += c0805("C30b", "10uF", 60, 48, 0, 2, "+3V3", 1, "GND")

# J19: RJ45 Magjack (right edge)
components += fp_header("J19", "RJ45_Magjack", "Connector_RJ:RJ45_Amphenol_RJHSE-5380", 92, 58, 90)
components += thru_pad("1", -3.5, -4.5, 1.4, 0.9, 60, "ETH_TXP")
components += thru_pad("2", -2.5, -4.5, 1.4, 0.9, 61, "ETH_TXN")
components += thru_pad("3", -1.5, -4.5, 1.4, 0.9, 62, "ETH_RXP")
components += thru_pad("4", -0.5, -4.5, 1.4, 0.9, 0, "")
components += thru_pad("5", 0.5, -4.5, 1.4, 0.9, 0, "")
components += thru_pad("6", 1.5, -4.5, 1.4, 0.9, 63, "ETH_RXN")
components += thru_pad("7", 2.5, -4.5, 1.4, 0.9, 0, "")
components += thru_pad("8", 3.5, -4.5, 1.4, 0.9, 0, "")
components += thru_pad("S1", -5.5, 0, 2.4, 1.6, 1, "GND")
components += thru_pad("S2", 5.5, 0, 2.4, 1.6, 1, "GND")
components += fp_end()

# =====================================================================
# U15: SIM7600E-H + U16: TPS7A2033 LDO (bottom area: x=38-55, y=62-78)
# =====================================================================
components += """
  ;; =====================================================================
  ;; U15: SIM7600E-H Cellular + U16: TPS7A2033 LDO
  ;; =====================================================================
"""
# U16: TPS7A2033 SOT-23-5 (3.8V LDO for SIM7600)
components += fp_header("U16", "TPS7A2033", "Package_TO_SOT_SMD:SOT-23-5", 42, 62)
components += smd_pad("1", -1.1, -0.95, 1.0, 0.6, 2, "+3V3")   # IN
components += smd_pad("2", -1.1, 0, 1.0, 0.6, 1, "GND")        # GND
components += smd_pad("3", -1.1, 0.95, 1.0, 0.6, 2, "+3V3")    # EN (tied to IN)
components += smd_pad("4", 1.1, 0.95, 1.0, 0.6, 0, "")         # NR
components += smd_pad("5", 1.1, -0.95, 1.0, 0.6, 6, "VBAT_SIM") # OUT
components += fp_end()

# LDO caps
components += c0402("C35", "1uF", 38, 62, 90, 2, "+3V3", 1, "GND")

# SIM7600 VBAT bulk caps
components += fp_header("C31", "100uF", "Capacitor_SMD:C_1206_3216Metric", 48, 62)
components += smd_pad("1", -1.5, 0, 1.2, 1.8, 6, "VBAT_SIM")
components += smd_pad("2", 1.5, 0, 1.2, 1.8, 1, "GND")
components += fp_end()

components += fp_header("C32", "470uF", "Capacitor_Tantalum_SMD:CP_EIA-7343-31_Kemet-D", 54, 62)
components += smd_pad("1", -2.9, 0, 2.0, 2.4, 6, "VBAT_SIM")
components += smd_pad("2", 2.9, 0, 2.0, 2.4, 1, "GND")
components += fp_end()

# U15: SIM7600E-H LCC module (large: ~30x30mm)
components += fp_header("U15", "SIM7600E-H", "WeatherStation:SIM7600EH_LCC", 48, 72)
# Simplified castellated pad module
components += smd_pad("1", -14, -4, 1.5, 0.8, 55, "CELL_RX")   # RXD
components += smd_pad("2", -14, -2, 1.5, 0.8, 54, "CELL_TX")   # TXD
components += smd_pad("3", -14, 0, 1.5, 0.8, 56, "CELL_PWRKEY")
components += smd_pad("4", -14, 2, 1.5, 0.8, 57, "CELL_STATUS")
components += smd_pad("5", -14, 4, 1.5, 0.8, 58, "CELL_DTR")
components += smd_pad("6", -14, 6, 1.5, 0.8, 59, "CELL_RI")
components += smd_pad("7", 14, -6, 1.5, 0.8, 6, "VBAT_SIM")   # VBAT
components += smd_pad("8", 14, -4, 1.5, 0.8, 1, "GND")         # GND
components += smd_pad("9", 14, 0, 1.5, 0.8, 74, "LTE_ANT")    # ANT
components += smd_pad("10", 14, 2, 1.5, 0.8, 70, "SIM_VCC")
components += smd_pad("11", 14, 4, 1.5, 0.8, 71, "SIM_RST")
components += smd_pad("12", 14, 6, 1.5, 0.8, 72, "SIM_CLK")
components += smd_pad("13", 14, 8, 1.5, 0.8, 73, "SIM_DATA")
components += smd_pad("GND", 0, 0, 10, 8, 1, "GND")  # Center GND
components += fp_end()

# J20: Nano SIM card holder
components += fp_header("J20", "NanoSIM", "Connector:NanoSIM_Molex_78800", 70, 72)
components += smd_pad("C1", -3, -2, 1.2, 0.8, 70, "SIM_VCC")
components += smd_pad("C2", -3, 0, 1.2, 0.8, 71, "SIM_RST")
components += smd_pad("C3", -3, 2, 1.2, 0.8, 72, "SIM_CLK")
components += smd_pad("C5", 3, 0, 1.2, 0.8, 1, "GND")
components += smd_pad("C7", 3, -2, 1.2, 0.8, 73, "SIM_DATA")
components += fp_end()

# J21: LTE SMA antenna (right edge, below RJ45)
components += fp_header("J21", "SMA_LTE", "Connector_Coaxial:SMA_Amphenol_132289_EdgeMount", 100, 72, 270)
components += smd_pad("1", 0, 0, 1.5, 1.5, 74, "LTE_ANT")
components += smd_pad("2", -2.5, 0, 1.5, 1.5, 1, "GND")
components += smd_pad("3", 2.5, 0, 1.5, 1.5, 1, "GND")
components += fp_end()

# =====================================================================
# SENSOR/INDUSTRY CONNECTORS (right edge)
# =====================================================================
components += """
  ;; =====================================================================
  ;; SENSOR & INDUSTRY CONNECTORS
  ;; =====================================================================
"""

# J4: Anemometer RJ11
components += fp_header("J4", "ANEMOMETER", "Connector_RJ:RJ11_Amphenol_54601", 96, 32, 90)
components += thru_pad("1", -1.5, -3.5, 1.4, 0.9, 84, "ANEM_SIG")
components += thru_pad("2", -0.5, -3.5, 1.4, 0.9, 1, "GND")
components += thru_pad("3", 0.5, -3.5, 1.4, 0.9, 2, "+3V3")
components += thru_pad("4", 1.5, -3.5, 1.4, 0.9, 0, "")
components += fp_end()

# J5: Rain gauge RJ11
components += fp_header("J5", "RAIN_GAUGE", "Connector_RJ:RJ11_Amphenol_54601", 96, 42, 90)
components += thru_pad("1", -1.5, -3.5, 1.4, 0.9, 85, "RAIN_SIG")
components += thru_pad("2", -0.5, -3.5, 1.4, 0.9, 1, "GND")
components += thru_pad("3", 0.5, -3.5, 1.4, 0.9, 2, "+3V3")
components += thru_pad("4", 1.5, -3.5, 1.4, 0.9, 0, "")
components += fp_end()

# R9a/R9b: Wind/rain pull-ups
components += r0402("R9a", "10k", 90, 34, 0, 2, "+3V3", 84, "ANEM_SIG")
components += r0402("R9b", "10k", 90, 44, 0, 2, "+3V3", 85, "RAIN_SIG")

# J7: Debug UART header
components += fp_header("J7", "DEBUG_UART", "Connector_PinHeader_2.54mm:PinHeader_1x04_P2.54mm_Vertical", 42, 48)
components += thru_pad("1", -3.81, 0, 1.7, 1.0, 42, "UART1_TX")
components += thru_pad("2", -1.27, 0, 1.7, 1.0, 43, "UART1_RX")
components += thru_pad("3", 1.27, 0, 1.7, 1.0, 2, "+3V3")
components += thru_pad("4", 3.81, 0, 1.7, 1.0, 1, "GND")
components += fp_end()

# R11a/R11b: UART series protection
components += r0402("R11a", "100R", 38, 48, 90, 42, "UART1_TX", 0, "")
components += r0402("R11b", "100R", 40, 48, 90, 43, "UART1_RX", 0, "")

# Agriculture connectors (J8-J11) — right side bottom
jst3_y = 46
for ref, val, sig_net_id, sig_net in [
    ("J8", "SOIL_MOIST_1", 86, "SOIL1_SIG"),
    ("J9", "SOIL_MOIST_2", 87, "SOIL2_SIG"),
    ("J10", "SOIL_TEMP", 88, "SOIL_TEMP_DATA"),
    ("J11", "LEAF_WET", 89, "LEAF_WET_SIG"),
]:
    jst3_y += 4
    components += fp_header(ref, val, "Connector_JST:JST_PH_B3B-PH-K_1x03_P2.00mm_Vertical", 80, jst3_y)
    components += thru_pad("1", -2.0, 0, 1.7, 0.8, 2, "+3V3")
    components += thru_pad("2", 0, 0, 1.7, 0.8, 1, "GND")
    components += thru_pad("3", 2.0, 0, 1.7, 0.8, sig_net_id, sig_net)
    components += fp_end()

# Solar connectors (J12-J15) — top-right
jst_y = 8
for ref, val, sig_net_id, sig_net in [
    ("J12", "PYRANOMETER", 90, "PYRANO_OUT"),
    ("J13", "PANEL_TEMP_F", 91, "PANEL_TEMP1_DATA"),
    ("J14", "PANEL_TEMP_B", 92, "PANEL_TEMP2_DATA"),
    ("J15", "POWER_PULSE", 93, "POWER_PULSE"),
]:
    components += fp_header(ref, val, "Connector_JST:JST_PH_B3B-PH-K_1x03_P2.00mm_Vertical", 78, jst_y)
    components += thru_pad("1", -2.0, 0, 1.7, 0.8, 2, "+3V3")
    components += thru_pad("2", 0, 0, 1.7, 0.8, 1, "GND")
    components += thru_pad("3", 2.0, 0, 1.7, 0.8, sig_net_id, sig_net)
    components += fp_end()
    jst_y += 3

# R9c: Power pulse pull-up, R10a/R10b: 1-Wire pull-ups, R12: leaf wetness pull-down
components += r0402("R9c", "10k", 82, 17, 0, 2, "+3V3", 93, "POWER_PULSE")
components += r0402("R10a", "4.7k", 82, 54, 0, 2, "+3V3", 88, "SOIL_TEMP_DATA")
components += r0402("R10b", "4.7k", 82, 11, 0, 2, "+3V3", 91, "PANEL_TEMP1_DATA")
components += r0402("R12", "10k", 82, 62, 0, 89, "LEAF_WET_SIG", 1, "GND")

# J22: MicroSD card holder
components += fp_header("J22", "MicroSD", "Connector:MicroSD_Molex_5031821852", 28, 48)
components += smd_pad("1", -5.5, -3.5, 0.8, 1.5, 26, "SD_CS")
components += smd_pad("2", -4.4, -3.5, 0.8, 1.5, 20, "SPI2_MOSI")
components += smd_pad("3", -3.3, -3.5, 0.8, 1.5, 1, "GND")
components += smd_pad("4", -2.2, -3.5, 0.8, 1.5, 2, "+3V3")
components += smd_pad("5", -1.1, -3.5, 0.8, 1.5, 22, "SPI2_SCK")
components += smd_pad("6", 0, -3.5, 0.8, 1.5, 1, "GND")
components += smd_pad("7", 1.1, -3.5, 0.8, 1.5, 21, "SPI2_MISO")
components += smd_pad("CD", 5.5, -3.5, 0.8, 1.5, 27, "SD_DET")
components += smd_pad("S1", -6.5, 4, 1.0, 1.5, 1, "GND")
components += smd_pad("S2", 6.5, 4, 1.0, 1.5, 1, "GND")
components += fp_end()

# SD card decoupling + detect pull-up
components += c0402("C38", "100nF", 24, 46, 0, 2, "+3V3", 1, "GND")
components += r0402("R27", "10k", 34, 46, 0, 2, "+3V3", 27, "SD_DET")

# D6: Status LED + R5
components += fp_header("D6", "LED_Green", "LED_SMD:LED_0603_1608Metric", 28, 44)
components += smd_pad("1", -0.75, 0, 0.8, 0.8, 1, "GND")
components += smd_pad("2", 0.75, 0, 0.8, 0.8, 96, "STATUS_LED")
components += fp_end()
components += r0402("R5", "330R", 24, 44, 0, 96, "STATUS_LED", 0, "")

# Append to file
with open(OUT, 'a') as f:
    f.write(components)

print("Part 3 written: Communication modules + connectors")
