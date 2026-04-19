# Weather Station — Comprehensive Cost Estimate

> **Date**: 2026-02-28 | **Rev**: 1.0 | **Currency**: USD
> **Basis**: Single unit prototype pricing. Volume discounts noted where applicable.

---

## 1. Electronic Components (Bill of Materials)

### 1.1 Microcontroller & Modules

| Ref | Component | Qty | Unit Price | Extended | Supplier |
|-----|-----------|-----|-----------|----------|----------|
| U1 | ESP32-S3-WROOM-1-N16R8I (Industrial) | 1 | $4.20 | $4.20 | Mouser |
| U8 | RFM95W-868S2 LoRa Module (SX1276) | 1 | $9.50 | $9.50 | Mouser |
| | **Subtotal — MCU & Modules** | | | **$13.70** | |

### 1.2 Sensors — Primary

| Ref | Component | Qty | Unit Price | Extended | Supplier |
|-----|-----------|-----|-----------|----------|----------|
| U4 | BME280 Temp/Humidity/Pressure | 1 | $3.50 | $3.50 | Mouser |
| U5 | AS5600-ASOM Magnetic Encoder (wind dir) | 1 | $2.80 | $2.80 | Mouser |
| U6 | SI1145-A10-GMR UV/IR/Vis Light | 1 | $3.20 | $3.20 | DigiKey |
| U7 | BH1750FVI-TR Ambient Light | 1 | $1.50 | $1.50 | Mouser |
| — | Anemometer SEN-15901 (wind speed) | 1 | $45.00 | $45.00 | SparkFun |
| — | Rain Gauge SEN-15902 (tipping bucket) | 1 | $25.00 | $25.00 | SparkFun |
| — | N52 Diametric Magnet 6mm (for AS5600) | 1 | $2.00 | $2.00 | Amazon |
| | **Subtotal — Primary Sensors** | | | **$83.00** | |

### 1.3 Sensors — Agriculture Module (Optional)

| Ref | Component | Qty | Unit Price | Extended | Supplier |
|-----|-----------|-----|-----------|----------|----------|
| — | Capacitive Soil Moisture Sensor v2.0 | 2 | $3.50 | $7.00 | Amazon |
| — | DS18B20 Waterproof Probe (soil temp) | 1 | $4.00 | $4.00 | Amazon |
| — | Leaf Wetness Sensor (resistive) | 1 | $8.00 | $8.00 | DigiKey |
| | **Subtotal — Agriculture Sensors** | | | **$19.00** | |

### 1.4 Sensors — Solar Module (Optional)

| Ref | Component | Qty | Unit Price | Extended | Supplier |
|-----|-----------|-----|-----------|----------|----------|
| — | ML8511 Pyranometer Breakout | 1 | $6.50 | $6.50 | SparkFun |
| — | DS18B20 Waterproof Probe (panel front) | 1 | $4.00 | $4.00 | Amazon |
| — | DS18B20 Waterproof Probe (panel back) | 1 | $4.00 | $4.00 | Amazon |
| — | S0 Pulse Power Meter Interface | 1 | $12.00 | $12.00 | — |
| | **Subtotal — Solar Sensors** | | | **$26.50** | |

### 1.5 Power Management ICs

| Ref | Component | Qty | Unit Price | Extended | Supplier |
|-----|-----------|-----|-----------|----------|----------|
| U2 | TP4056 LiPo Charger IC (SOIC-8) | 1 | $0.30 | $0.30 | LCSC |
| U2A | DW01A-G Battery Protection IC | 1 | $0.15 | $0.15 | LCSC |
| Q1 | FS8205A Dual MOSFET | 1 | $0.20 | $0.20 | LCSC |
| U3 | AMS1117-3.3 LDO Regulator | 1 | $0.25 | $0.25 | LCSC |
| F1 | MF-MSMF050-2 Polyfuse 500mA (1206) | 1 | $0.35 | $0.35 | Bourns |
| D1 | SS14 Schottky Diode (solar protection) | 1 | $0.15 | $0.15 | LCSC |
| D2 | SMAJ5.0A TVS Diode (USB ESD) | 1 | $0.30 | $0.30 | LCSC |
| | **Subtotal — Power ICs** | | | **$1.70** | |

### 1.6 Passive Components

| Ref | Component | Qty | Unit Price | Extended | Supplier |
|-----|-----------|-----|-----------|----------|----------|
| R1-R12 | Resistors, 0402, various values | 22 | $0.01 | $0.22 | LCSC |
| C1-C5 | Capacitors, 0402/0805, various | 16 | $0.02 | $0.32 | LCSC |
| D3-D6 | LEDs 0603 (Red/Green/Blue/Green) | 4 | $0.05 | $0.20 | LCSC |
| SW1-SW2 | PTS645 Tactile Buttons (6mm) | 2 | $0.20 | $0.40 | C&K |
| | **Subtotal — Passives** | | | **$1.14** | |

### 1.7 Connectors

| Ref | Component | Qty | Unit Price | Extended | Supplier |
|-----|-----------|-----|-----------|----------|----------|
| J1 | USB4105-GF-A USB-C Receptacle | 1 | $0.85 | $0.85 | DigiKey |
| J2-J3, J15 | JST B2B-PH-K-S 2-pin | 3 | $0.25 | $0.75 | JST |
| J4-J5 | RJ11 4P4C Amphenol 54601-908LF | 2 | $1.20 | $2.40 | Amphenol |
| J6 | SMA Female Edge Mount 132289 | 1 | $2.50 | $2.50 | Amphenol |
| J7 | Pin Header 1x04 2.54mm | 1 | $0.15 | $0.15 | Generic |
| J8-J11, J13-J14 | JST B3B-PH-K-S 3-pin | 6 | $0.30 | $1.80 | JST |
| J12 | JST B4B-PH-K-S 4-pin | 1 | $0.35 | $0.35 | JST |
| — | JST PH crimped wire harnesses (matching) | 12 | $1.00 | $12.00 | — |
| | **Subtotal — Connectors** | | | **$20.80** | |

### 1.8 Power Supply & Battery

| Component | Qty | Unit Price | Extended | Supplier |
|-----------|-----|-----------|----------|----------|
| 18650 LiPo Battery 3000mAh (protected cell) | 1 | $8.00 | $8.00 | Amazon |
| 18650 Battery Holder (PCB or wired) | 1 | $1.50 | $1.50 | Amazon |
| Solar Panel 6V 2W (110x136mm) | 1 | $6.00 | $6.00 | Amazon |
| USB-C Cable (1m, for programming/charging) | 1 | $3.00 | $3.00 | — |
| **Subtotal — Power** | | | **$18.50** | |

### 1.9 RF/Antenna

| Component | Qty | Unit Price | Extended | Supplier |
|-----------|-----|-----------|----------|----------|
| LoRa 868/915MHz Antenna (SMA, 3dBi) | 1 | $4.50 | $4.50 | Amazon |
| SMA Pigtail cable (if needed) | 1 | $2.00 | $2.00 | — |
| **Subtotal — Antenna** | | | **$6.50** | |

### BOM Cost Summary

| Category | Base Station | + Agriculture | + Solar | Full Build |
|----------|-------------|---------------|---------|------------|
| MCU & Modules | $13.70 | $13.70 | $13.70 | $13.70 |
| Primary Sensors | $83.00 | $83.00 | $83.00 | $83.00 |
| Agriculture Sensors | — | $19.00 | — | $19.00 |
| Solar Sensors | — | — | $26.50 | $26.50 |
| Power ICs | $1.70 | $1.70 | $1.70 | $1.70 |
| Passives | $1.14 | $1.14 | $1.14 | $1.14 |
| Connectors | $20.80 | $20.80 | $20.80 | $20.80 |
| Power & Battery | $18.50 | $18.50 | $18.50 | $18.50 |
| Antenna/RF | $6.50 | $6.50 | $6.50 | $6.50 |
| **BOM Total** | **$145.34** | **$164.34** | **$171.84** | **$190.84** |

---

## 2. PCB Design & Fabrication

### 2.1 PCB Design (One-Time NRE)

| Item | Cost | Notes |
|------|------|-------|
| Schematic capture (KiCad) | $0 | Done in-house (this project) |
| PCB layout (4-layer, impedance-controlled) | $0 | Done in-house (this project) |
| Design rule check (DRC) | $0 | KiCad automated |
| Gerber/drill file generation | $0 | KiCad export |
| **Subtotal — PCB Design** | **$0** | In-house, no external cost |

> **Note**: If outsourcing PCB layout to a professional designer:
> - 4-layer board, 100x80mm, ~54 components = **$300–$600** one-time
> - Impedance-controlled layout adds ~$100–$200

### 2.2 PCB Fabrication

| Parameter | Specification | Cost Impact |
|-----------|--------------|-------------|
| Board size | 100 x 80mm | — |
| Layers | 4 | Higher than 2-layer |
| Material | FR4 Tg170 | +$5 vs standard FR4 |
| Finish | ENIG | +$8 vs HASL |
| Impedance control | 50ohm + 90ohm | +$10 per order |
| Min trace/space | 0.15mm/0.15mm | Standard capability |

| Quantity | Per-Board Cost | Total | Supplier |
|----------|---------------|-------|----------|
| 5 pcs (prototype) | $12.00 | $60.00 | JLCPCB |
| 10 pcs | $8.50 | $85.00 | JLCPCB |
| 50 pcs | $4.20 | $210.00 | JLCPCB |
| 100 pcs | $3.00 | $300.00 | JLCPCB/PCBWay |

### 2.3 Solder Stencil

| Item | Cost | Notes |
|------|------|-------|
| Steel stencil (framed, 0.12mm) | $25.00 | Reusable for all boards |
| Frameless stencil | $8.00 | Budget option |

### 2.4 PCB Assembly (SMT)

| Option | Per-Board | Setup Fee | Notes |
|--------|-----------|-----------|-------|
| Hand assembly (DIY) | $0 | $0 | Requires hot air station + stencil |
| JLCPCB SMT Assembly (5 pcs) | $18.00 | $8.00 | Basic parts from LCSC stock |
| PCBWay Assembly (5 pcs) | $22.00 | $30.00 | Extended parts library |
| Professional PCBA (10+ pcs) | $15.00 | $50.00 | Full turnkey |

### PCB Cost Summary (Prototype — 5 boards)

| Item | Cost |
|------|------|
| PCB fabrication (5 pcs, 4-layer ENIG) | $60.00 |
| Solder stencil (framed) | $25.00 |
| SMT assembly (5 pcs, JLCPCB) | $98.00 |
| Shipping (DHL Express) | $25.00 |
| **PCB Total (5 prototypes)** | **$208.00** |
| **Per-board PCB cost** | **$41.60** |

---

## 3. Testing & Calibration

### 3.1 Test Equipment (One-Time Investment)

| Equipment | Cost | Notes |
|-----------|------|-------|
| Digital multimeter (basic) | $30.00 | Voltage, current, continuity |
| USB-C power meter/tester | $15.00 | Verify charging current |
| Logic analyzer (8-ch, Saleae clone) | $12.00 | I2C/SPI/UART debugging |
| USB-UART adapter (CP2102/CH340) | $5.00 | Serial console access |
| Hot air rework station | $60.00 | SMD rework & repair |
| Soldering iron (Pinecil/TS100) | $45.00 | Fine-pitch hand soldering |
| **Subtotal — Test Equipment** | **$167.00** | One-time purchase |

> If already equipped, skip this section. These are basic maker/lab tools.

### 3.2 Calibration Reference Standards

| Item | Cost | Notes |
|------|------|-------|
| NIST-traceable thermometer (±0.1°C) | $45.00 | Temperature reference |
| NIST-traceable hygrometer (±2% RH) | $65.00 | Humidity reference |
| Barometric reference (local airport METAR) | $0 | Free public data |
| UV reference meter | $85.00 | SI1145 calibration |
| Lux meter (calibrated) | $35.00 | BH1750 calibration |
| Anemometer reference (or lookup table) | $0 | Manufacturer-provided curve |
| **Subtotal — Calibration Standards** | **$230.00** | One-time purchase |

### 3.3 Per-Unit Testing & Calibration Labor

| Test Step | Time (min) | Cost @$25/hr | Notes |
|-----------|-----------|--------------|-------|
| Visual inspection (AOI check) | 5 | $2.08 | Solder joints, components |
| Power-on test (3.3V rail, current) | 5 | $2.08 | USB-C power, LED check |
| Firmware flash (esptool) | 3 | $1.25 | Flash via USB-C |
| I2C sensor scan & verify | 5 | $2.08 | All 4 sensor addresses |
| Temperature calibration (3-point) | 30 | $12.50 | 0°C, 25°C, 50°C reference |
| Humidity calibration (2-point) | 20 | $8.33 | 33% and 75% salt solutions |
| Pressure calibration (1-point) | 5 | $2.08 | Compare to local METAR |
| Light sensor calibration | 10 | $4.17 | BH1750 + SI1145 vs reference |
| Wind sensor verification | 10 | $4.17 | Pulse counting, direction sweep |
| Rain gauge verification | 5 | $2.08 | Tip counter check |
| WiFi/BLE connectivity test | 5 | $2.08 | Scan, connect, data transfer |
| LoRa range test (basic) | 10 | $4.17 | TX/RX packet exchange |
| Battery charging test | 10 | $4.17 | Charge current, protection cutoff |
| Write calibration offsets to NVS | 5 | $2.08 | Store per-unit cal data |
| Final QA sign-off | 5 | $2.08 | Log serial number, pass/fail |
| **Total per unit** | **133 min** | **$54.84** | ~2.2 hours |

### 3.4 Agriculture Module Calibration (Optional)

| Test Step | Time (min) | Cost @$25/hr |
|-----------|-----------|--------------|
| Soil moisture sensor calibration (wet/dry) | 20 | $8.33 |
| Soil temperature probe verification | 10 | $4.17 |
| Leaf wetness threshold tuning | 15 | $6.25 |
| **Agriculture cal total** | **45 min** | **$18.75** |

### 3.5 Solar Module Calibration (Optional)

| Test Step | Time (min) | Cost @$25/hr |
|-----------|-----------|--------------|
| Pyranometer calibration (outdoor reference) | 30 | $12.50 |
| Panel temperature probe verification (x2) | 15 | $6.25 |
| Power meter pulse verification | 10 | $4.17 |
| **Solar cal total** | **55 min** | **$22.92** |

### Testing & Calibration Cost Summary

| Item | One-Time | Per-Unit |
|------|----------|----------|
| Test equipment | $167.00 | — |
| Calibration standards | $230.00 | — |
| Base station testing & cal | — | $54.84 |
| Agriculture module cal | — | $18.75 |
| Solar module cal | — | $22.92 |
| **Total (base)** | **$397.00** | **$54.84** |
| **Total (full build)** | **$397.00** | **$96.51** |

---

## 4. Outdoor Station Enclosure & Fabrication

### 4.1 Main Electronics Enclosure

| Item | Cost | Notes |
|------|------|-------|
| IP65 ABS junction box (150x100x60mm) | $12.00 | Gewiss GW44205 or equivalent |
| IP67 upgrade (polycarbonate, clear lid) | $18.00 | If transparent lid needed |
| PG7 cable glands (IP68) | 4 x $1.50 = $6.00 | Sensor cables |
| PG9 cable glands (IP68) | 2 x $2.00 = $4.00 | Power/antenna cables |
| Gore-Tex vent plug (M12) | $8.00 | Pressure equalization |
| M3 brass standoffs 8mm + screws | 4 x $0.30 = $1.20 | PCB mounting |
| M3 nylon lock nuts + washers | 4 x $0.10 = $0.40 | Anti-vibration |
| Silicone gasket sealant (tube) | $6.00 | Extra sealing on lid |
| **Subtotal — Main Enclosure** | **$37.60** | IP65 version |
| **Subtotal — Main Enclosure (IP67)** | **$43.60** | Polycarbonate upgrade |

### 4.2 Sensor Radiation Shield (Stevenson Screen)

| Item | Cost | Notes |
|------|------|-------|
| 3D-printed radiation shield (PETG/ASA) | $8.00 | Material cost, ~100g filament |
| Commercial radiation shield (Davis 7714) | $25.00 | Professional option |
| UV-stable white spray paint (if 3D printed) | $5.00 | Krylon UV-resistant |
| M4 stainless mounting hardware | $3.00 | Bolts, nuts, washers |
| **Subtotal — Radiation Shield (DIY)** | **$16.00** | |
| **Subtotal — Radiation Shield (commercial)** | **$28.00** | |

### 4.3 Mounting Pole & Hardware

| Item | Cost | Notes |
|------|------|-------|
| Galvanized steel pole (1.5m, 32mm OD) | $15.00 | Main mounting mast |
| Pole ground plate / flange mount | $8.00 | Base mounting |
| U-bolts for enclosure mounting (stainless) | 2 x $3.00 = $6.00 | Box to pole |
| Pole cap / weather cap | $2.00 | Top seal |
| Anemometer mounting arm (L-bracket, SS) | $8.00 | Extend from pole top |
| Rain gauge mounting bracket | $5.00 | Level mount, clear of obstructions |
| Cable ties (UV-resistant, black) | $4.00 | Pack of 100 |
| Cable conduit / spiral wrap (2m) | $5.00 | Outdoor cable protection |
| **Subtotal — Mounting** | **$53.00** | |

### 4.4 Painting & Surface Treatment

| Item | Cost | Notes |
|------|------|-------|
| Primer spray (metal, rust-inhibiting) | $8.00 | For pole and brackets |
| UV-resistant white topcoat spray (x2 cans) | $12.00 | Reflective, weatherproof |
| Stainless steel polish / passivation | $5.00 | For SS hardware |
| Anti-seize compound (small tube) | $6.00 | Threaded connections |
| **Subtotal — Paint & Treatment** | **$31.00** | |

### 4.5 Weatherproofing & Sealing

| Item | Cost | Notes |
|------|------|-------|
| Conformal coating (Humiseal 1A33, 55mL) | $22.00 | PCB protection spray |
| Self-amalgamating tape (for SMA connector) | $5.00 | Waterproof RF connection |
| Heat shrink tubing assortment | $4.00 | Wire splice protection |
| Dielectric grease (small tube) | $4.00 | Connector waterproofing |
| Marine-grade silicone sealant | $8.00 | External cable entry sealing |
| **Subtotal — Weatherproofing** | **$43.00** | |

### 4.6 Solar Panel Mounting

| Item | Cost | Notes |
|------|------|-------|
| Adjustable tilt bracket (small, aluminum) | $10.00 | Solar panel angle adjustment |
| Stainless mounting screws for panel | $3.00 | — |
| MC4 to bare wire adapter (if panel uses MC4) | $4.00 | Solar panel connection |
| **Subtotal — Solar Mount** | **$17.00** | |

### Outdoor Station Cost Summary

| Category | Cost |
|----------|------|
| Main enclosure (IP65) | $37.60 |
| Radiation shield (DIY) | $16.00 |
| Mounting pole & hardware | $53.00 |
| Painting & surface treatment | $31.00 |
| Weatherproofing & sealing | $43.00 |
| Solar panel mounting | $17.00 |
| **Outdoor Station Total** | **$197.60** |

---

## 5. Packaging, Shipping & Miscellaneous

### 5.1 Product Packaging (Per Unit)

| Item | Cost | Notes |
|------|------|-------|
| Corrugated shipping box (outer, 400x300x200mm) | $3.50 | Double-wall, sturdy |
| Inner product box (custom printed, optional) | $5.00 | Branded packaging |
| PE foam insert (die-cut, protective) | $4.00 | Custom cutouts for components |
| Anti-static bags (for PCB + cables) | $0.50 | ESD protection |
| Desiccant packets (x3) | $0.30 | Moisture protection |
| Bubble wrap / air pillows (void fill) | $1.00 | Shock protection |
| Cable wrap / velcro ties | $0.50 | Tidy cable management |
| **Subtotal — Packaging** | **$14.80** | Per unit |

### 5.2 Documentation & Accessories (Per Unit)

| Item | Cost | Notes |
|------|------|-------|
| Quick start guide (printed, color, A5) | $1.50 | Folded insert |
| Pinout / wiring diagram card | $0.50 | Laminated reference card |
| USB-C cable (1m) | $3.00 | For charging/programming |
| Spare cable glands (2x PG7) | $3.00 | Replacement |
| Spare JST PH connectors (4-pack) | $2.00 | Pre-crimped pigtails |
| Mounting screws assortment bag | $2.00 | SS screws, nuts, washers |
| **Subtotal — Docs & Accessories** | **$12.00** | Per unit |

### 5.3 Shipping Costs

| Destination | Weight (~3 kg) | Cost |
|-------------|----------------|------|
| Domestic (US, ground) | 3 kg | $12.00 |
| Domestic (US, express) | 3 kg | $25.00 |
| International (standard) | 3 kg | $35.00 |
| International (express) | 3 kg | $55.00 |

### 5.4 Miscellaneous / Consumables

| Item | Cost | Notes |
|------|------|-------|
| Solder paste (10g syringe, SAC305) | $8.00 | Shared across builds |
| Flux (rosin pen) | $5.00 | Shared |
| IPA cleaning (500mL bottle) | $6.00 | PCB cleaning |
| Kapton tape | $4.00 | Masking during conformal coat |
| Label printer labels (serial numbers) | $3.00 | QR code + serial |
| Wire (22AWG silicone, 5 colors, 5m each) | $8.00 | Wiring harnesses |
| Heatshrink assortment | $4.00 | Wire protection |
| **Subtotal — Consumables** | **$38.00** | Shared across ~10 units |
| **Per-unit consumables** | **$3.80** | |

### Packaging & Misc Cost Summary (Per Unit)

| Category | Cost |
|----------|------|
| Product packaging | $14.80 |
| Documentation & accessories | $12.00 |
| Shipping (domestic ground) | $12.00 |
| Consumables (per-unit share) | $3.80 |
| **Packaging & Misc Total** | **$42.60** |

---

## 6. Grand Total Summary

### Per-Unit Cost Breakdown (Prototype, 1–5 units)

| Category | Base Station | Full Build (Agri+Solar) |
|----------|-------------|------------------------|
| **Electronic Components (BOM)** | $145.34 | $190.84 |
| **PCB Fabrication & Assembly** | $41.60 | $41.60 |
| **Testing & Calibration** | $54.84 | $96.51 |
| **Outdoor Station & Enclosure** | $197.60 | $197.60 |
| **Packaging, Shipping & Misc** | $42.60 | $42.60 |
| | | |
| **TOTAL PER UNIT** | **$481.98** | **$569.15** |

### One-Time Costs (NRE / Setup)

| Item | Cost |
|------|------|
| Test equipment | $167.00 |
| Calibration reference standards | $230.00 |
| Solder stencil (reusable) | $25.00 |
| **Total One-Time NRE** | **$422.00** |

### True Cost for First Unit

| | Base | Full Build |
|---|------|------------|
| Per-unit cost | $481.98 | $569.15 |
| One-time NRE | $422.00 | $422.00 |
| **First Unit Total** | **$903.98** | **$991.15** |

### Volume Pricing Estimate

| Volume | Per-Unit (Base) | Per-Unit (Full) | Notes |
|--------|----------------|-----------------|-------|
| 1 unit | $903.98 | $991.15 | Includes NRE |
| 5 units | $566.38 | $653.55 | NRE amortized |
| 10 units | $524.18 | $611.35 | PCB cost drops ~30% |
| 50 units | $420.00 | $500.00 | BOM volume discounts ~15% |
| 100 units | $360.00 | $435.00 | Assembly line efficiency |

> Volume pricing assumes bulk component discounts, amortized NRE, and
> assembly efficiency gains. Actual pricing depends on supplier MOQs
> and assembly partner rates.

---

## 7. Cost Optimization Opportunities

| Opportunity | Savings | Trade-off |
|-------------|---------|-----------|
| Use HASL finish instead of ENIG | $8/board | Reduced corrosion resistance |
| 2-layer PCB (remove impedance control) | $5/board | Lose LoRa performance |
| Commercial-grade ESP32 (drop -I suffix) | $1.50/unit | 0°C lower limit |
| Source sensors from LCSC/AliExpress | $15–25/unit | Longer lead time, QC risk |
| 3D-print enclosure (PETG/ASA) | $20 savings | Less IP protection |
| Skip conformal coating | $22 savings | Reduced outdoor durability |
| DIY assembly (no PCBA service) | $18/board | Requires soldering skill |
| Bundle anemometer + rain gauge kit | $10–15 savings | Buy weather instrument kit |

---

*This estimate is based on single-unit / small-batch prototype pricing as of
February 2026. Prices may vary by supplier, region, and order timing. All costs
are estimates and should be verified with supplier quotes before procurement.*
