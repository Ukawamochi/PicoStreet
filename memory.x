/*
 * Raspberry Pi RP2040 memory layout
 */
MEMORY
{
  FLASH (rx)  : ORIGIN = 0x10000000, LENGTH = 2048K
  RAM   (rwx) : ORIGIN = 0x20000000, LENGTH = 264K
}

/* This is required by cortex-m-rt to set the stack top */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);

