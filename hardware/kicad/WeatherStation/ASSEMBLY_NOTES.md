# Weather Station PCB — Assembly & Manufacturing Notes

## PCB Specifications

| Parameter | Value |
|-----------|-------|
| Board Dimensions | 100mm x 80mm |
| Layer Count | 4 |
| Stackup | Signal / GND / Power / Signal |
| Thickness | 1.6mm |
| Material | FR4 Tg170 |
| Copper Weight | 1oz (35um) all layers |
| Surface Finish | ENIG (for outdoor corrosion resistance) |
| Solder Mask | Green, both sides |
| Silkscreen | White, both sides |
| Min Trace/Space | 0.15mm / 0.15mm |
| Min Drill | 0.3mm (laser via), 0.4mm (mechanical) |
| Impedance Control | Yes (50ohm LoRa, 90ohm USB diff) |
| IPC Class | 2 (Standard) |
| UL Rating | UL 94V-0 |

## ESP32-S3 Module Selection

### Commercial vs Industrial Grade

| Variant | Ordering Code | Temp Range | Use Case |
|---------|--------------|------------|----------|
| **Commercial** | ESP32-S3-WROOM-1-N16R8 | 0°C to +85°C | Indoor / lab prototyping |
| **Industrial** | ESP32-S3-WROOM-1-N16R8**I** | -40°C to +85°C | **Outdoor weather station** |

> **RECOMMENDATION**: For production weather stations, always use the **-I (Industrial)**
> variant. The 12% cost premium (~$1.50) is negligible versus field failure costs.

### Why NOT Industrial Grade Enough?

The ESP32-S3-WROOM-1 module, even in its industrial variant, has limitations for
harsh environments:

1. **Not Automotive Grade** — Not AEC-Q100 qualified
2. **Not MIL-SPEC** — Not tested to MIL-STD-810
3. **No IP Rating** — The module itself has no ingress protection
4. **Humidity** — Rated 10-90% RH non-condensing only

### Mitigation for Outdoor Deployment

| Risk | Mitigation |
|------|-----------|
| Water ingress | IP65+ ABS enclosure with cable glands |
| Condensation | Conformal coating (Humiseal 1A33) on PCB |
| UV degradation | UV-stabilized enclosure, no direct sun on PCB |
| Temperature extremes | Industrial (-I) variant + thermal design |
| Lightning/surge | TVS diodes on all external connectors |
| Corrosion | ENIG finish + conformal coating |
| Vibration | M3 standoffs with lock washers, strain relief on cables |

## Layer Stackup Detail

```
┌─────────────────────────────────────────┐
│  F.SilkS   — Component labels          │
│  F.Mask    — Solder mask (green)        │
│  F.Cu      — Top copper (signal)  35um  │  ← Components side
│  Prepreg   — 0.2mm FR4                  │
│  In1.Cu    — GND plane (solid)    35um  │  ← Unbroken ground reference
│  Core      — 1.065mm FR4                │
│  In2.Cu    — Power plane          35um  │  ← 3V3 pour + VBAT island
│  Prepreg   — 0.2mm FR4                  │
│  B.Cu      — Bottom copper (signal)35um │  ← Connectors, some passives
│  B.Mask    — Solder mask (green)        │
│  B.SilkS   — Bottom labels             │
└─────────────────────────────────────────┘
Total: 1.6mm
```

## Critical Layout Rules

### Antenna Keep-Out Zone
- **Area**: 30mm x 12mm centered on top edge of PCB
- **Constraint**: No copper on ANY layer (including ground planes)
- No components within this zone
- ESP32-S3 module positioned so onboard antenna extends into this clear area

### LoRa RF Trace
- 50-ohm controlled impedance from SX1276 RFIO pin to SMA connector
- Trace width: calculate based on stackup (approx 0.28mm for 50ohm on 0.2mm prepreg)
- Keep ground plane continuous under RF trace
- Surround SMA pad with ground stitching vias (0.3mm drill, 0.6mm ring, 1mm spacing)
- PI matching network between SX1276 and antenna

### USB Differential Pair
- 90-ohm differential impedance
- Route on top layer with ground reference on In1.Cu
- Keep pair tightly coupled, match lengths within 0.1mm
- Avoid crossing splits in reference plane

### Power Distribution
- In2.Cu: Main 3V3 copper pour covering >80% of board
- VBAT island on In2.Cu for battery-direct components only
- Decoupling caps within 3mm of every IC power pin
- Use thermal relief on ground vias under exposed pads

## Assembly Sequence

1. **Solder Paste** — Stencil print (0.12mm thickness)
2. **Place SMD Bottom** — If any bottom-side components
3. **Reflow Bottom** — Peak 245°C, SAC305
4. **Place SMD Top** — All ICs and passives
5. **Reflow Top** — Peak 245°C, SAC305
6. **Place THT** — Connectors (JST-PH, RJ11, SMA, pin headers)
7. **Wave/Selective Solder** — THT components
8. **Manual** — ESP32-S3 module (if not reflowed), USB-C
9. **Clean** — IPA wash to remove flux residue
10. **Inspect** — AOI + visual inspection
11. **Program** — Flash firmware via USB-C or UART header
12. **Test** — Functional test (see test procedure below)
13. **Conformal Coat** — Humiseal 1A33, avoid connectors and antenna

## Functional Test Procedure

```
1. Visual inspection — no shorts, missing components, tombstones
2. Power-on test:
   a. Connect USB-C, verify 3.3V rail (U3 output) = 3.30V ±0.05V
   b. Verify charge LED (D3) illuminates with battery connected
   c. Measure current draw: ~80mA idle (WiFi off), ~180mA (WiFi active)
3. Firmware flash:
   a. Hold BOOT, press RESET, release BOOT
   b. Flash via esptool: esptool.py --chip esp32s3 write_flash 0x0 firmware.bin
4. Sensor check:
   a. I2C scan: should find 0x76 (BME280), 0x36 (AS5600), 0x60 (SI1145), 0x23 (BH1750)
   b. Read temperature (should be ~25°C room temp)
   c. Read pressure (should be ~1013 hPa at sea level)
5. Communication check:
   a. WiFi: scan for APs, verify connection
   b. BLE: scan from phone, verify GATT advertisement
   c. LoRa: transmit test packet, verify on receiver
   d. UART: connect terminal at 115200 baud, verify console output
6. GPIO check:
   a. Trigger anemometer input — verify pulse count
   b. Trigger rain gauge input — verify tip count
   c. Toggle status LED — verify GPIO2 output
7. Industry connector check (if populated):
   a. Agriculture: verify ADC readings on GPIO7, GPIO17, GPIO18
   b. Agriculture: verify 1-Wire on GPIO16
   c. Solar: verify ADC on GPIO7, 1-Wire on GPIO16/GPIO19
   d. Solar: verify pulse input on GPIO20
```

## Enclosure Recommendations

| Parameter | Specification |
|-----------|--------------|
| Material | ABS or polycarbonate, UV-stabilized |
| IP Rating | IP65 minimum (IP67 preferred) |
| Size | 150mm x 100mm x 60mm (internal) |
| Color | White or light gray (minimize solar heating) |
| Mounting | DIN rail or wall mount with stainless hardware |
| Cable Entry | PG7/PG9 cable glands (IP68) for sensor cables |
| Ventilation | Gore-Tex vent plug (allows pressure equalization, blocks water) |
| Temperature | -40°C to +85°C rated |

## Conformal Coating Map

```
  ┌──────────────────────────────────┐
  │ ████ COAT ████  ░░NO COAT░░     │  ← Antenna zone: NO COAT
  │ ████████████████████████████████ │
  │ ████ ESP32-S3 ████  COAT  █████ │  ← Module body: COAT
  │ ████████████████████████████████ │
  │ █ LoRa █  ████████  █ Sensors █ │
  │ █ COAT █  ████████  █  COAT   █ │
  │ ████████  ████████  ███████████ │
  │ ░░SMA░░  ████████  ░░░░░░░░░░░ │  ← SMA: NO COAT
  │ █ Power █  ██████  ░Connectors░ │  ← JST/RJ11: NO COAT
  │ █ COAT  █  ██████  ░░NO COAT ░░ │
  │ ░░BAT░░  ░░USB-C░░  ░░UART░░░░ │  ← All edge connectors: NO COAT
  └──────────────────────────────────┘

  ████ = Apply conformal coating
  ░░░░ = Mask off (no coating)
```

## Ordering Checklist

- [ ] Verify industrial temp ESP32-S3 (-I suffix) in BOM
- [ ] Verify 4-layer stackup specified in fabrication notes
- [ ] Verify impedance control requested (50ohm + 90ohm)
- [ ] Verify ENIG surface finish specified
- [ ] Verify Tg170 FR4 specified
- [ ] Generate Gerber files (RS-274X format)
- [ ] Generate drill files (Excellon format)
- [ ] Generate pick-and-place file (CSV with X/Y/Rotation)
- [ ] Include IPC-D-356 netlist for e-test
- [ ] Review DFM report from fabricator before ordering
