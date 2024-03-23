#[derive(Copy, Clone, Debug)]
pub enum SysNum {
    Clone,
    Close,
    Creat,
    ExitGroup,
    Fork,
    GetPid,
    GetRandom,
    Open,
    OpenAt,
    Mmap,
    Munmap,
    NewFstatAt,
    Read,
    VFork,
    Wait,
    Write,

    Other(u64),
}

pub type SyscallResult = Result<u64, nix::errno::Errno>;

impl From<u64> for SysNum {
    fn from(num: u64) -> Self {
        match num {
            0 => SysNum::Read,
            1 => SysNum::Write,
            2 => SysNum::Open,
            3 => SysNum::Close,
            9 => SysNum::Mmap,
            11 => SysNum::Munmap,
            39 => SysNum::GetPid,
            56 => SysNum::Clone,
            57 => SysNum::Fork,
            58 => SysNum::VFork,
            61 => SysNum::Wait,
            85 => SysNum::Creat,
            231 => SysNum::ExitGroup,
            257 => SysNum::OpenAt,
            262 => SysNum::NewFstatAt,
            318 => SysNum::GetRandom,
            _ => Self::Other(num),
        }
    }
}

impl From<SysNum> for u64 {
    fn from(num: SysNum) -> u64 {
        match num {
            SysNum::Read => 0,
            SysNum::Write => 1,
            SysNum::Open => 2,
            SysNum::Close => 3,
            SysNum::Mmap => 9,
            SysNum::Munmap => 11,
            SysNum::GetPid => 39,
            SysNum::Clone => 56,
            SysNum::Fork => 57,
            SysNum::VFork => 58,
            SysNum::Wait => 61,
            SysNum::Creat => 85,
            SysNum::ExitGroup => 231,
            SysNum::OpenAt => 257,
            SysNum::NewFstatAt => 262,
            SysNum::GetRandom => 318,
            SysNum::Other(num) => num,
        }
    }
}
