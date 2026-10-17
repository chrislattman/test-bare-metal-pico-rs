/* Memory layout for QEMU's mps2-an521 machine (secure aliases), used instead of ../rp2350/memory.x
 * when building with the qemu feature. The CPU loads its initial SP and PC from the vector
 * table at 0x10000000.
 */
MEMORY {
    /* ZBT SSRAM1 (4 MiB) stands in for flash */
    FLASH : ORIGIN = 0x10000000, LENGTH = 2048K
    /* ZBT SSRAM2/3 */
    RAM : ORIGIN = 0x38000000, LENGTH = 4096K
}

SECTIONS {
    /* Boot ROM metadata from main.rs (unused in QEMU), kept where the RP2350 layout puts it */
    .start_block : ALIGN(4)
    {
        KEEP(*(.start_block));
    } > FLASH
} INSERT AFTER .vector_table;

_stext = ADDR(.start_block) + SIZEOF(.start_block);
