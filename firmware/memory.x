/* ESP32-S3 Memory Layout */
/* Linker script for weather station firmware */

MEMORY
{
    /* Internal SRAM - 512 KB */
    IRAM   (rwx) : ORIGIN = 0x40370000, LENGTH = 64K    /* Instruction RAM */
    DRAM   (rw)  : ORIGIN = 0x3FC88000, LENGTH = 448K   /* Data RAM */

    /* External PSRAM - 8 MB */
    PSRAM  (rw)  : ORIGIN = 0x3C000000, LENGTH = 8M

    /* External Flash - 16 MB */
    /* Partitioned as follows: */
    /* 0x000000 - 0x00FFFF : Bootloader (64 KB) */
    /* 0x010000 - 0x010FFF : Partition table (4 KB) */
    /* 0x011000 - 0x016FFF : NVS (24 KB) */
    /* 0x017000 - 0x018FFF : OTA data (8 KB) */
    /* 0x019000 - 0x418FFF : App partition 0 (4 MB) */
    /* 0x419000 - 0x818FFF : App partition 1 / OTA (4 MB) */
    /* 0x819000 - 0xFFFFFF : Data storage (7.9 MB) */
    IROM   (rx)  : ORIGIN = 0x42000000, LENGTH = 4M     /* App code in flash */
    DROM   (r)   : ORIGIN = 0x3C000000, LENGTH = 4M     /* Read-only data */
}

REGION_ALIAS("REGION_TEXT", IROM);
REGION_ALIAS("REGION_RODATA", DROM);
REGION_ALIAS("REGION_DATA", DRAM);
REGION_ALIAS("REGION_BSS", DRAM);
REGION_ALIAS("REGION_STACK", DRAM);
REGION_ALIAS("REGION_HEAP", DRAM);

/* Stack size for each core */
_stack_size_core0 = 8K;
_stack_size_core1 = 8K;

/* Heap size */
_heap_size = 384K;
