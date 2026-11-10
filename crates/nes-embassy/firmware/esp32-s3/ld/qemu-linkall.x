/* QEMU linker script: same as esp-hal's linkall.x but with alias-qemu.x
 * routing ROTEXT->IRAM and RODATA->DRAM (see alias-qemu.x). The other
 * INCLUDEs resolve via the linker's -L search path to esp-hal's files.
 */
INCLUDE "memory.x"
INCLUDE "alias-qemu.x"
INCLUDE "esp32s3.x"
INCLUDE "hal-defaults.x"
