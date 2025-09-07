/*
 * Raspberry Pi RP2040 memory layout
 */
MEMORY {
  /* RP2040 external QSPI flash mapped at XIP */
  BOOT2 (rx)  : ORIGIN = 0x10000000, LENGTH = 0x100       /* 256-byte second-stage bootloader */
  FLASH (rx)  : ORIGIN = 0x10000100, LENGTH = 2048K - 0x100
  RAM   (rwx) : ORIGIN = 0x20000000, LENGTH = 264K
}

/*
 * BOOT2 セクションをフラッシュ先頭に固定配置。
 * cortex-m-rt の .vector_table の直前に挿入する。
 */
SECTIONS {
  .boot2 ORIGIN(BOOT2) : {
    KEEP(*(.boot2));
  } > BOOT2
} INSERT BEFORE .vector_table;

/* cortex-m-rt が使用するスタックトップ */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);
