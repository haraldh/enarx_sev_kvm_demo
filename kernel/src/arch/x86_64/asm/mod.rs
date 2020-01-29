global_asm!(include_str!("pvh_notes.s"));
global_asm!(include_str!("pvh_ram32.s"));
global_asm!(include_str!("pvh_ram64.s"));

global_asm!(include_str!("entry_ram64.s"));
global_asm!(include_str!("syscall.s"));
