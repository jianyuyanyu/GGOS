use super::*;
use crate::utils::Registers;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::vec::Vec;
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::structures::paging::PhysFrame;
use x86_64::VirtAddr;
use xmas_elf::ElfFile;

once_mutex!(pub PROCESS_MANAGER: ProcessManager);
guard_access_fn! {
    pub get_process_manager(PROCESS_MANAGER: ProcessManager)
}

pub struct ProcessManager {
    /// pid of the current running process
    cur_pid: ProcessId,
    processes: Vec<Process>,
    exit_code: BTreeMap<ProcessId, isize>,
}

impl ProcessManager {
    pub fn new(init: Process) -> Self {
        let mut processes = Vec::<Process>::new();
        let exit_code = BTreeMap::new();
        processes.push(init);
        Self {
            cur_pid: ProcessId(0),
            processes,
            exit_code,
        }
    }

    fn current_mut(&mut self) -> &mut Process {
        self.processes.iter_mut().find(|x| x.pid() == self.cur_pid).unwrap()
    }

    fn pid_mut(&mut self, pid: ProcessId) -> &mut Process {
        self.processes.iter_mut().find(|x| x.pid() == pid).unwrap()
    }

    pub fn current(&self) -> &Process {
        self.processes
            .iter()
            .find(|x| x.pid() == self.cur_pid)
            .unwrap()
    }

    pub fn current_pid(&self) -> ProcessId {
        self.cur_pid
    }

    pub fn add_child(&mut self, child: ProcessId) {
        self.current_mut().add_child(child);
    }

    pub fn save_current(&mut self, regs: &mut Registers, sf: &mut InterruptStackFrame) {
        let current = self.current_mut();
        if current.is_running() {
            current.tick();
            current.save(regs, sf);
        }
        // debug!("Paused process #{}", self.cur_pid);
    }

    fn get_next_pos(&self) -> usize {
        let cur_pos = self
            .processes
            .iter()
            .position(|x| x.pid() == self.cur_pid)
            .unwrap_or(0);

        let mut next_pos = (cur_pos + 1) % self.processes.len();

        while self.processes[next_pos].status() != ProgramStatus::Ready {
            next_pos = (next_pos + 1) % self.processes.len();
        }

        next_pos
    }

    pub fn wait_pid(&mut self, pid: ProcessId) -> isize {
        if self.exit_code.contains_key(&pid) {
            *self.exit_code.get(&pid).unwrap()
        } else {
            -1
        }
    }

    pub fn still_alive(&self, pid: ProcessId) -> bool {
        self.processes.iter().any(|x| x.pid() == pid)
    }

    pub fn switch_next(&mut self, regs: &mut Registers, sf: &mut InterruptStackFrame) {
        let pos = self.get_next_pos();
        let p = &mut self.processes[pos];

        // debug!("Next process {} #{}", p.name(), p.pid());
        if p.pid() == self.cur_pid {
            // the next process to be resumed is the same as the current one
            p.resume();
        } else {
            // switch to next process
            p.restore(regs, sf);
            self.cur_pid = p.pid();
        }
    }

    fn get_kernel_page_table(&self) -> PhysFrame {
        let proc = self.processes.get(0).unwrap();
        proc.page_table_addr()
    }

    pub fn open(&mut self, path: &str, mode: u8) -> Option<u8> {
        let res = match path {
            "/dev/random" => Resource::Random(fs::Random::new(
                crate::utils::clock::now().timestamp() as u64,
            )),
            path => {
                let file = crate::filesystem::try_get_file(path, fs::Mode::try_from(mode).unwrap());

                if file.is_err() {
                    return None;
                }

                Resource::File(file.unwrap())
            }
        };

        trace!("Opening {}...\n{:#?}", path, &res);

        let fd = self.current_mut().open(res);

        Some(fd)
    }

    pub fn close(&mut self, fd: u8) -> bool {
        if fd < 4 {
            false // stdin, stdout, stderr, exec file are reserved
        } else {
            self.current_mut().close(fd)
        }
    }

    pub fn spawn(
        &mut self,
        elf: &ElfFile,
        name: String,
        parent: ProcessId,
        proc_data: Option<ProcessData>,
    ) -> ProcessId {
        let mut p = Process::new(
            &mut *crate::memory::get_frame_alloc_for_sure(),
            name,
            parent,
            self.get_kernel_page_table(),
            proc_data,
        );
        p.pause();
        trace!(
            "Init stack frame with: \n    entry: {:#x}\n    stack: {:#x}",
            elf.header.pt2.entry_point(),
            STACK_BOT + STACK_SIZE
        );
        p.init_stack_frame(
            VirtAddr::new_truncate(elf.header.pt2.entry_point()),
            VirtAddr::new_truncate(STACK_BOT + STACK_SIZE),
        );
        p.init_elf(&elf);
        trace!("{:#?}", &p);
        // info!("Spawn process: {}#{}", p.name(), p.pid());
        // info!("Spawn process:\n\n{:?}\n", p);
        let pid = p.pid();
        self.processes.push(p);
        pid
    }

    pub fn spawn_kernel_thread(
        &mut self,
        entry: VirtAddr,
        stack_top: VirtAddr,
        name: String,
        parent: ProcessId,
        proc_data: Option<ProcessData>,
    ) -> ProcessId {
        let mut p = Process::new(
            &mut *crate::memory::get_frame_alloc_for_sure(),
            name,
            parent,
            self.get_kernel_page_table(),
            proc_data,
        );
        p.pause();
        p.init_stack_frame(entry, stack_top);
        info!("Spawn process: {}#{}", p.name(), p.pid());
        let pid = p.pid();
        self.processes.push(p);
        pid
    }

    pub fn print_process_list(&self) {
        let mut output = String::from("  PID | PPID | Name         |    Ticks | Status\n");
        for p in self.processes.iter() {
            output = output + format!("{}\n", p).as_str();
        }
        print!("{}", output);
    }

    pub fn fork(&mut self) {
        let mut p = self.current_mut().fork();
        p.pause();
        self.processes.push(p);
    }

    pub fn kill_self(&mut self, ret: isize) {
        self.kill(self.cur_pid, ret);
    }

    pub fn unblock(&mut self, pid: ProcessId) {
        self.processes.iter_mut().for_each(|p| {
            if p.pid() == pid {
                p.pause()
            }
        });
    }

    pub fn block(&mut self, pid: ProcessId) {
        self.processes.iter_mut().for_each(|p| {
            if p.pid() == pid {
                p.block()
            }
        });
    }

    pub fn kill(&mut self, pid: ProcessId, ret: isize) {
        trace!("Killing process #{} with ret code: {}", pid, ret);

        let p = self.processes.iter().find(|x| x.pid() == pid);

        if p.is_none() {
            warn!("Process #{} not found", pid);
            return;
        }

        let p = p.unwrap();

        let parent = p.parent();
        let children = p.children();
        let cur_page_table_addr = p.page_table_addr().start_address().as_u64();

        self.processes.iter_mut().for_each(|x| {
            if children.contains(&x.pid()) {
                x.set_parent(parent);
            }
        });

        let not_drop_page_table = self.processes.iter().any(|x| {
            x.page_table_addr().start_address().as_u64() == cur_page_table_addr && pid != x.pid()
        });

        if not_drop_page_table {
            self.pid_mut(pid).not_drop_page_table();
        }

        self.processes.retain(|p| p.pid() != pid);

        if let Some(proc) = self.processes.iter_mut().find(|x| x.pid() == parent) {
            proc.remove_child(pid);
        }

        if ret == !0xdeadbeef {
            // killed by other process
            return;
        }

        if let Err(_) = self.exit_code.try_insert(pid, ret) {
            error!("Process #{} already exited", pid);
        }
    }
}
