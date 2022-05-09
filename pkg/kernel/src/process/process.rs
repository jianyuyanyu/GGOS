use super::ProgramStatus;
use crate::filesystem::StdIO;
use crate::memory::BootInfoFrameAllocator;
use crate::utils::{Registers, RegistersValue, Resource};
use crate::memory::physical_to_virtual;
use core::intrinsics::copy_nonoverlapping;
use x86_64::structures::paging::{OffsetPageTable, PhysFrame, PageTable};
use x86_64::structures::idt::{InterruptStackFrameValue, InterruptStackFrame};
use x86_64::structures::paging::FrameAllocator;
use x86_64::registers::control::{Cr3, Cr3Flags};
use x86_64::registers::rflags::RFlags;
use alloc::string::String;
use alloc::vec::Vec;
use x86_64::VirtAddr;
use alloc::collections::btree_map::BTreeMap;

pub struct Process {
    pid: u16,
    regs: RegistersValue,
    name: String,
    parent: u16,
    status: ProgramStatus,
    ticks_passed: usize,
    children: Vec::<u16>,
    stack_frame: InterruptStackFrameValue,
    page_table_addr: (PhysFrame, Cr3Flags),
    page_table: Option<OffsetPageTable<'static>>,
    proc_data: ProcessData
}

#[derive(Clone, Debug, Default)]
pub struct ProcessData {
    env: BTreeMap<String, String>,
    file_handles: BTreeMap<u8, Resource>
}

impl ProcessData {
    pub fn new() -> Self {
        let env = BTreeMap::new();
        let mut file_handles = BTreeMap::new();
        // stdin, stdout, stderr
        file_handles.insert(0, Resource::Console(StdIO::new()));
        file_handles.insert(1, Resource::Console(StdIO::new()));
        file_handles.insert(2, Resource::Console(StdIO::new()));
        Self { env, file_handles }
    }

    pub fn set_env(mut self, key: &str, val: &str) -> Self {
        self.env.insert(key.into(), val.into());
        self
    }
}

impl Process {
    pub fn pid(&self) -> u16 {
        self.pid
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn tick(&mut self) {
        self.ticks_passed += 1;
    }

    pub fn pause(&mut self) {
        self.status = ProgramStatus::Ready;
    }

    pub fn resume(&mut self) {
        self.status = ProgramStatus::Running;
    }

    pub fn set_page_table_with_cr3(&mut self) {
        self.page_table_addr = Cr3::read();
    }

    pub fn is_running(&self) -> bool {
        self.status == ProgramStatus::Running
    }

    pub fn env(&self, key: &str) -> Option<String> {
        self.proc_data.env.get(key).cloned()
    }

    pub fn set_env(&mut self, key: &str, val: &str) {
        self.proc_data.env.insert(key.into(), val.into());
    }

    pub fn save(&mut self, regs: &mut Registers, sf: &mut InterruptStackFrame) {
        self.regs = unsafe{ regs.as_mut().read().clone() };
        self.stack_frame = unsafe{ sf.as_mut().read().clone() };
        self.status = ProgramStatus::Ready;
    }

    pub fn restore(&mut self, regs: &mut Registers, sf: &mut InterruptStackFrame) {
        unsafe {
            regs.as_mut().write(self.regs);
            sf.as_mut().write(self.stack_frame);
            Cr3::write(self.page_table_addr.0, self.page_table_addr.1)
        }
        self.status = ProgramStatus::Running;
    }

    pub fn init_stack_frame(&mut self, entry: VirtAddr, stack_top: VirtAddr) {
        self.stack_frame.stack_pointer = stack_top;
        self.stack_frame.instruction_pointer = entry;
        self.stack_frame.cpu_flags = (RFlags::IOPL_HIGH | RFlags::IOPL_LOW | RFlags::INTERRUPT_FLAG).bits();
    }

    pub fn new(
        frame_alloc: &mut BootInfoFrameAllocator,
        pid: u16, name: String, parent: u16,
        proc_data: Option<ProcessData>
    ) -> Self {
        // 1. alloc a page table for process
        let page_table_addr = frame_alloc.allocate_frame()
            .expect("Cannot alloc page table for new process.");
        trace!("Alloc page table for {}: {:?}", name, page_table_addr);

        // 2. copy current page table to new page table
        unsafe {
            copy_nonoverlapping::<PageTable>(
                Cr3::read().0.start_address().as_u64() as *mut PageTable,
                page_table_addr.start_address().as_u64() as *mut PageTable,
                1
            );
        }

        // 3. create page table object
        let page_table_raw = unsafe {
            (physical_to_virtual(page_table_addr.start_address().as_u64())
                as *mut PageTable)
            .as_mut()
        }.unwrap();

        let page_table = unsafe {
            OffsetPageTable::new(
                page_table_raw,
                VirtAddr::new_truncate(crate::memory::PHYSICAL_OFFSET as u64),
            )
        };

        // 4. create context
        let status = ProgramStatus::Created;
        let stack_frame = InterruptStackFrameValue {
            instruction_pointer: VirtAddr::new_truncate(0),
            code_segment: 8,
            cpu_flags: 0,
            stack_pointer: VirtAddr::new_truncate(0),
            stack_segment: 0,
        };
        let regs = RegistersValue::default();
        let ticks_passed = 0;

        debug!("New process {}#{} created.", name, pid);

        // 3. create process object
        Self {
            pid,
            name,
            parent,
            status,
            ticks_passed,
            stack_frame,
            regs,
            page_table_addr: (page_table_addr, Cr3::read().1),
            page_table: Some(page_table),
            children: Vec::new(),
            proc_data: proc_data.unwrap_or_default()
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        // TODO: deallocate memory
    }
}

impl core::fmt::Debug for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "Process {{\n")?;
        write!(f, "    pid: {},\n", self.pid)?;
        write!(f, "    name: {},\n", self.name)?;
        write!(f, "    parent: {},\n", self.parent)?;
        write!(f, "    status: {:?},\n", self.status)?;
        write!(f, "    ticks_passed: {},\n", self.ticks_passed)?;
        write!(f, "    children: {:?}\n", self.children)?;
        write!(f, "    page_table_addr: {:?},\n", self.page_table_addr)?;
        write!(f, "    stack_top: 0x{:016x},\n", self.stack_frame.stack_pointer.as_u64())?;
        write!(f, "    cpu_flags: 0x{:04x},\n", self.stack_frame.cpu_flags)?;
        write!(f, "    instruction_pointer: 0x{:016x}\n", self.stack_frame.instruction_pointer.as_u64())?;
        write!(f, "}}")?;
        Ok(())
    }
}

impl core::fmt::Display for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "#{:3} | {:10} | {}", self.pid, self.name, self.ticks_passed)?;
        Ok(())
    }
}
