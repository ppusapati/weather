/* STM32F407VGT6 Memory Layout */
/* Single-MCU weather station firmware */
/* 1 MB Flash, 128 KB SRAM + 64 KB CCM RAM */

MEMORY
{
    /* Main Flash — 1 MB total */
    /* Sector 0-3:   16 KB each (64 KB) — Bootloader */
    /* Sector 4:     64 KB             — OTA metadata */
    /* Sector 5-7:   128 KB each       — App partition (384 KB) */
    /* Sector 8-11:  128 KB each       — OTA partition (512 KB) */
    FLASH (rx)  : ORIGIN = 0x08000000, LENGTH = 1024K

    /* Main SRAM — 128 KB */
    RAM   (rwx) : ORIGIN = 0x20000000, LENGTH = 128K

    /* Core-Coupled Memory (CCM) RAM — 64 KB */
    /* Used for stack and time-critical data */
    CCMRAM (rw) : ORIGIN = 0x10000000, LENGTH = 64K
}

/* Stack in CCM for deterministic access */
_stack_start = ORIGIN(CCMRAM) + LENGTH(CCMRAM);

/* Heap allocation — 48 KB from main SRAM */
_heap_size = 48K;
