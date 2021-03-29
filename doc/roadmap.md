# Memory management #

- High-memory allocator
  - (DONE) Allocate virtual addresses
  - (DONE) Map high-memory virtual addresses to highmem PA
  - (DONE) Use a guard to ensure high-memory unmapping and deallocation
- (WORKEDAROUND) General purpose allocator

# Process management #

# Devices #

# User interface #

- Kernel-space keyboard support:
  - Keymaps:
    1. Define a keymap format (or use standard one?)
    2. Generate keymap files for US-QWERTY and FR-AZERTY first
    3. Map physical scan-codes into logical keys
- Graphical terminal:
  - (DONE) Pixmap font generator
  - (DONE) Pixmap font loader
  - (DONE) Text renderer

# User-space #
