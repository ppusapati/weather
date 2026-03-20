#!/usr/bin/env python3
"""Part 4: Board outline, mounting holes, fiducials, zones, silkscreen, and close file."""

OUT = "/home/user/weather/hardware/kicad/WeatherStation/WeatherStation_mfg.kicad_pcb"

content = """
  ;; =====================================================================
  ;; BOARD OUTLINE — 100mm x 80mm rounded rectangle (5mm corner radius)
  ;; =====================================================================

  (gr_line (start 5 0) (end 95 0) (layer "Edge.Cuts") (width 0.1) (tstamp "e0000001-0000-0000-0000-000000000001"))
  (gr_arc (start 95 0) (mid 98.536 1.464) (end 100 5) (layer "Edge.Cuts") (width 0.1) (tstamp "e0000001-0000-0000-0000-000000000002"))
  (gr_line (start 100 5) (end 100 75) (layer "Edge.Cuts") (width 0.1) (tstamp "e0000001-0000-0000-0000-000000000003"))
  (gr_arc (start 100 75) (mid 98.536 78.536) (end 95 80) (layer "Edge.Cuts") (width 0.1) (tstamp "e0000001-0000-0000-0000-000000000004"))
  (gr_line (start 95 80) (end 5 80) (layer "Edge.Cuts") (width 0.1) (tstamp "e0000001-0000-0000-0000-000000000005"))
  (gr_arc (start 5 80) (mid 1.464 78.536) (end 0 75) (layer "Edge.Cuts") (width 0.1) (tstamp "e0000001-0000-0000-0000-000000000006"))
  (gr_line (start 0 75) (end 0 5) (layer "Edge.Cuts") (width 0.1) (tstamp "e0000001-0000-0000-0000-000000000007"))
  (gr_arc (start 0 5) (mid 1.464 1.464) (end 5 0) (layer "Edge.Cuts") (width 0.1) (tstamp "e0000001-0000-0000-0000-000000000008"))

  ;; =====================================================================
  ;; MOUNTING HOLES (M3) — 4 corners
  ;; =====================================================================

  (footprint "MountingHole:MountingHole_3.2mm_M3_Pad_Via" (layer "F.Cu")
    (tstamp "f0000001-0000-0000-0000-000000000001")
    (at 5 5)
    (property "Reference" "H1" (at 0 -3) (layer "F.SilkS") (effects (font (size 1 1) (thickness 0.15))))
    (property "Value" "MountingHole_M3" (at 0 3) (layer "F.Fab") (effects (font (size 1 1) (thickness 0.15))))
    (pad "1" thru_hole circle (at 0 0) (size 6.0 6.0) (drill 3.2) (layers "*.Cu" "*.Mask") (net 1 "GND"))
  )
  (footprint "MountingHole:MountingHole_3.2mm_M3_Pad_Via" (layer "F.Cu")
    (tstamp "f0000001-0000-0000-0000-000000000002")
    (at 95 5)
    (property "Reference" "H2" (at 0 -3) (layer "F.SilkS") (effects (font (size 1 1) (thickness 0.15))))
    (property "Value" "MountingHole_M3" (at 0 3) (layer "F.Fab") (effects (font (size 1 1) (thickness 0.15))))
    (pad "1" thru_hole circle (at 0 0) (size 6.0 6.0) (drill 3.2) (layers "*.Cu" "*.Mask") (net 1 "GND"))
  )
  (footprint "MountingHole:MountingHole_3.2mm_M3_Pad_Via" (layer "F.Cu")
    (tstamp "f0000001-0000-0000-0000-000000000003")
    (at 5 75)
    (property "Reference" "H3" (at 0 -3) (layer "F.SilkS") (effects (font (size 1 1) (thickness 0.15))))
    (property "Value" "MountingHole_M3" (at 0 3) (layer "F.Fab") (effects (font (size 1 1) (thickness 0.15))))
    (pad "1" thru_hole circle (at 0 0) (size 6.0 6.0) (drill 3.2) (layers "*.Cu" "*.Mask") (net 1 "GND"))
  )
  (footprint "MountingHole:MountingHole_3.2mm_M3_Pad_Via" (layer "F.Cu")
    (tstamp "f0000001-0000-0000-0000-000000000004")
    (at 95 75)
    (property "Reference" "H4" (at 0 -3) (layer "F.SilkS") (effects (font (size 1 1) (thickness 0.15))))
    (property "Value" "MountingHole_M3" (at 0 3) (layer "F.Fab") (effects (font (size 1 1) (thickness 0.15))))
    (pad "1" thru_hole circle (at 0 0) (size 6.0 6.0) (drill 3.2) (layers "*.Cu" "*.Mask") (net 1 "GND"))
  )

  ;; =====================================================================
  ;; FIDUCIALS (3 points for pick-and-place alignment)
  ;; =====================================================================

  (footprint "Fiducial:Fiducial_1mm_Mask2mm" (layer "F.Cu")
    (tstamp "f0000002-0000-0000-0000-000000000001")
    (at 8 8)
    (property "Reference" "FID1" (at 0 -2.5) (layer "F.SilkS") (effects (font (size 0.8 0.8) (thickness 0.12))))
    (property "Value" "Fiducial" (at 0 2.5) (layer "F.Fab") (effects (font (size 0.8 0.8) (thickness 0.12))))
    (pad "1" smd circle (at 0 0) (size 1 1) (layers "F.Cu" "F.Mask"))
  )
  (footprint "Fiducial:Fiducial_1mm_Mask2mm" (layer "F.Cu")
    (tstamp "f0000002-0000-0000-0000-000000000002")
    (at 92 8)
    (property "Reference" "FID2" (at 0 -2.5) (layer "F.SilkS") (effects (font (size 0.8 0.8) (thickness 0.12))))
    (property "Value" "Fiducial" (at 0 2.5) (layer "F.Fab") (effects (font (size 0.8 0.8) (thickness 0.12))))
    (pad "1" smd circle (at 0 0) (size 1 1) (layers "F.Cu" "F.Mask"))
  )
  (footprint "Fiducial:Fiducial_1mm_Mask2mm" (layer "F.Cu")
    (tstamp "f0000002-0000-0000-0000-000000000003")
    (at 8 72)
    (property "Reference" "FID3" (at 0 -2.5) (layer "F.SilkS") (effects (font (size 0.8 0.8) (thickness 0.12))))
    (property "Value" "Fiducial" (at 0 2.5) (layer "F.Fab") (effects (font (size 0.8 0.8) (thickness 0.12))))
    (pad "1" smd circle (at 0 0) (size 1 1) (layers "F.Cu" "F.Mask"))
  )

  ;; =====================================================================
  ;; GROUND STITCHING VIAS (perimeter + near RF sections)
  ;; =====================================================================
"""

# Generate ground stitching vias
via_id = 1
for x in range(10, 96, 5):
    content += f'  (via (at {x} 3) (size 0.6) (drill 0.3) (layers "F.Cu" "B.Cu") (net 1) (tstamp "gstitch-{via_id:04d}"))\n'
    via_id += 1
    content += f'  (via (at {x} 77) (size 0.6) (drill 0.3) (layers "F.Cu" "B.Cu") (net 1) (tstamp "gstitch-{via_id:04d}"))\n'
    via_id += 1

for y in range(10, 76, 5):
    content += f'  (via (at 3 {y}) (size 0.6) (drill 0.3) (layers "F.Cu" "B.Cu") (net 1) (tstamp "gstitch-{via_id:04d}"))\n'
    via_id += 1
    content += f'  (via (at 97 {y}) (size 0.6) (drill 0.3) (layers "F.Cu" "B.Cu") (net 1) (tstamp "gstitch-{via_id:04d}"))\n'
    via_id += 1

# Extra stitching around SMA connectors and RF areas
for x, y in [(5, 36), (5, 40), (5, 46), (5, 50), (15, 42), (18, 42)]:
    content += f'  (via (at {x} {y}) (size 0.6) (drill 0.3) (layers "F.Cu" "B.Cu") (net 1) (tstamp "gstitch-{via_id:04d}"))\n'
    via_id += 1

content += """
  ;; =====================================================================
  ;; COPPER ZONES
  ;; =====================================================================

  ;; In1.Cu: Solid GND plane (no splits)
  (zone (net 1) (net_name "GND") (layer "In1.Cu") (tstamp "zone-gnd-in1")
    (hatch edge 0.508)
    (connect_pads (clearance 0.2))
    (min_thickness 0.15)
    (fill yes (thermal_gap 0.3) (thermal_bridge_width 0.3))
    (polygon
      (pts
        (xy 0 0)
        (xy 100 0)
        (xy 100 80)
        (xy 0 80)
      )
    )
  )

  ;; In2.Cu: 3V3 power plane
  (zone (net 2) (net_name "+3V3") (layer "In2.Cu") (tstamp "zone-3v3-in2")
    (hatch edge 0.508)
    (connect_pads (clearance 0.2))
    (min_thickness 0.15)
    (fill yes (thermal_gap 0.3) (thermal_bridge_width 0.3))
    (polygon
      (pts
        (xy 0 0)
        (xy 100 0)
        (xy 100 80)
        (xy 0 80)
      )
    )
  )

  ;; F.Cu: GND pour (fills remaining area)
  (zone (net 1) (net_name "GND") (layer "F.Cu") (tstamp "zone-gnd-fcu") (priority 0)
    (hatch edge 0.508)
    (connect_pads (clearance 0.3))
    (min_thickness 0.15)
    (fill yes (thermal_gap 0.4) (thermal_bridge_width 0.4))
    (polygon
      (pts
        (xy 0 0)
        (xy 100 0)
        (xy 100 80)
        (xy 0 80)
      )
    )
  )

  ;; B.Cu: GND pour (fills remaining area)
  (zone (net 1) (net_name "GND") (layer "B.Cu") (tstamp "zone-gnd-bcu") (priority 0)
    (hatch edge 0.508)
    (connect_pads (clearance 0.3))
    (min_thickness 0.15)
    (fill yes (thermal_gap 0.4) (thermal_bridge_width 0.4))
    (polygon
      (pts
        (xy 0 0)
        (xy 100 0)
        (xy 100 80)
        (xy 0 80)
      )
    )
  )

  ;; =====================================================================
  ;; SILKSCREEN
  ;; =====================================================================

  (gr_text "P9E" (at 50 36) (layer "F.SilkS")
    (effects (font (size 2.5 2.5) (thickness 0.3) (bold yes)) (justify center))
  )
  (gr_text "P9E Microsystems Pvt Ltd" (at 50 39.5) (layer "F.SilkS")
    (effects (font (size 1 1) (thickness 0.15)) (justify center))
  )
  (gr_text "www.p9e.in" (at 50 42) (layer "F.SilkS")
    (effects (font (size 0.8 0.8) (thickness 0.12)) (justify center))
  )
  (gr_text "Weather Station v2.0" (at 50 45) (layer "F.SilkS")
    (effects (font (size 1.5 1.5) (thickness 0.2) (bold yes)) (justify center))
  )
  (gr_text "STM32F407VGT6 | Industrial -40C/+105C" (at 50 48) (layer "F.SilkS")
    (effects (font (size 0.8 0.8) (thickness 0.12)) (justify center))
  )

  ;; Back silkscreen
  (gr_text "P9E Microsystems Pvt Ltd\\nwww.p9e.in\\nWeather Station PCB Rev 2.0\\n4-Layer | FR4 Tg170 | ENIG\\nRoHS Compliant | IPC Class 2\\nConformal coat before deployment" (at 50 40) (layer "B.SilkS")
    (effects (font (size 1 1) (thickness 0.15)) (justify center mirror))
  )

  ;; =====================================================================
  ;; DESIGN NOTES (User.Comments layer)
  ;; =====================================================================

  (gr_text "DESIGN NOTES:\\n1. 4-layer stackup: Signal/GND/Power/Signal\\n2. In1.Cu = solid GND plane\\n3. In2.Cu = 3V3 pour + VBAT_SIM island\\n4. Controlled impedance: 50ohm (LoRa, WiFi, LTE), 90ohm diff (USB, Ethernet)\\n5. All decoupling caps within 3mm of IC power pins\\n6. Board finish: ENIG\\n7. Min trace/space: 0.15mm/0.15mm\\n8. Min drill: 0.3mm (laser), 0.4mm (mechanical)\\n9. Solder mask: green, both sides\\n10. Apply conformal coating for outdoor deployment"
    (at 110 20) (layer "Cmts.User")
    (effects (font (size 1.2 1.2) (thickness 0.15)) (justify left))
  )

  (gr_text "COMPONENT SUMMARY:\\nU10: STM32F407VGT6 (LQFP-100)\\nU11: MAX3485 RS485 (SOIC-8)\\nU12: ATWINC1500 WiFi (QFN-28)\\nU13: RN4870 BLE (Module)\\nU14: W5500 Ethernet (LQFP-48)\\nU15: SIM7600E-H Cellular (LCC)\\nU16: TPS7A2033 3.8V LDO (SOT-23-5)\\nU2: TP4056 Charger (SOIC-8)\\nU3: AMS1117-3.3 (SOT-223)\\nU4-U7: Environmental Sensors\\nU8: SX1276 LoRa (Module)\\n18 Connectors, 80+ total components"
    (at 110 60) (layer "Cmts.User")
    (effects (font (size 1.2 1.2) (thickness 0.15)) (justify left))
  )

  (gr_text "CONNECTOR PLACEMENT (Board Edge):\\nJ1 (USB-C): Bottom edge center\\nJ2 (Solar): Bottom-left\\nJ3 (Battery): Bottom-left\\nJ4 (Anemometer): Right edge\\nJ5 (Rain Gauge): Right edge\\nJ6 (SMA LoRa): Left edge\\nJ7 (UART Debug): Center\\nJ8-J11 (Agriculture): Right side\\nJ12-J15 (Solar): Top-right\\nJ16 (RS485): Top-right\\nJ18 (SMA WiFi): Left edge\\nJ19 (RJ45 Ethernet): Right edge\\nJ20 (Nano-SIM): Bottom-right\\nJ21 (SMA LTE): Right edge\\nJ22 (MicroSD): Center"
    (at 110 100) (layer "Cmts.User")
    (effects (font (size 1.2 1.2) (thickness 0.15)) (justify left))
  )

)
"""

with open(OUT, 'a') as f:
    f.write(content)

print("Part 4 written: Board outline, zones, silkscreen, design notes")
print(f"Manufacturing PCB complete: {OUT}")
