use nix::errno::Errno;

pub enum Operation {
    Open { path: String },
    Exit,
}

pub enum OperationResult {
    FileDescriptor(i32),
    Error(Errno),
}

pub enum SysNum {
    Close,
    Creat,
    ExitGroup,
    Open,
    OpenAt,
    Mmap,
    Munmap,
    Read,
    Write,

    Other(u64),
}

impl From<u64> for SysNum {
    fn from(num: u64) -> Self {
        match num {
            0 => SysNum::Read,
            1 => SysNum::Write,
            2 => SysNum::Open,
            3 => SysNum::Close,
            9 => SysNum::Mmap,
            11 => SysNum::Munmap,
            85 => SysNum::Creat,
            231 => SysNum::ExitGroup,
            257 => SysNum::OpenAt,
            _ => Self::Other(num),
        }
    }
}
