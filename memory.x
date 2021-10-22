MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes */
  /* These values correspond to the NRF52833 with Softdevices S140 7.0.1 */
  FLASH : ORIGIN = 0x00000000 + 112K, LENGTH = 512K - 112K
  RAM : ORIGIN = 0x20000000 + 64K, LENGTH = 128K - 64K
}
