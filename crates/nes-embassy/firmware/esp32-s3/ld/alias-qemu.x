/* QEMU workaround: the ESP32-S3 QEMU machine's ROM loader faults with a
 * LoadStoreError when it touches flash-mapped DROM/IROM (0x3c000020).
 * Tallow boots in the same QEMU because its image is pure internal RAM.
 * Route read-only sections into DRAM/IRAM so the image has no flash-mapped
 * segments at all. Real hardware keeps the default flash mapping.
 */
REGION_ALIAS("ROTEXT", iram_seg);
REGION_ALIAS("RWTEXT", iram_seg);
REGION_ALIAS("RODATA", dram_seg);
REGION_ALIAS("RWDATA", dram_seg);
REGION_ALIAS("RTC_FAST_RWTEXT", rtc_fast_seg);
REGION_ALIAS("RTC_FAST_RWDATA", rtc_fast_seg);
