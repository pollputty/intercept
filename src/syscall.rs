#[derive(Copy, Clone, Debug)]
pub enum SysNum {
    Access,
    ArchPRCTL,
    Brk,
    Clone,
    Close,
    Creat,
    Dup,
    Dup2,
    Execve,
    Exit,
    ExitGroup,
    Fcntl,
    Fork,
    Futex,
    GetEGID,
    GetEUID,
    GetGID,
    GetPGRP,
    GetPID,
    GetPPID,
    GetRandom,
    GetUID,
    IOCtl, // TODO?
    Kill,  // TOOD?
    LSeek,
    Open,
    OpenAt,
    Mmap,
    Mprotect,
    Munmap,
    NewFstatAt,
    Pipe2,
    PRead,
    PRLimit,
    PWrite,
    Read,
    Rseq,
    RTSigAction,
    RTSigProcmask,
    RTSigReturn,
    SetGID,
    SetPGID,
    SetRobustList,
    SetTIDAddress,
    SetUID,
    SysInfo, // TODO?
    Uname,   // TODO?
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
            8 => SysNum::LSeek,
            9 => SysNum::Mmap,
            10 => SysNum::Mprotect,
            11 => SysNum::Munmap,
            12 => SysNum::Brk,
            13 => SysNum::RTSigAction,
            14 => SysNum::RTSigProcmask,
            15 => SysNum::RTSigReturn,
            16 => SysNum::IOCtl,
            17 => SysNum::PRead,
            18 => SysNum::PWrite,
            21 => SysNum::Access,
            32 => SysNum::Dup,
            33 => SysNum::Dup2,
            39 => SysNum::GetPID,
            56 => SysNum::Clone,
            57 => SysNum::Fork,
            58 => SysNum::VFork,
            59 => SysNum::Execve,
            60 => SysNum::Exit,
            61 => SysNum::Wait,
            62 => SysNum::Kill,
            63 => SysNum::Uname,
            72 => SysNum::Fcntl,
            85 => SysNum::Creat,
            99 => SysNum::SysInfo,
            102 => SysNum::GetUID,
            104 => SysNum::GetGID,
            105 => SysNum::SetUID,
            106 => SysNum::SetGID,
            107 => SysNum::GetEUID,
            108 => SysNum::GetEGID,
            109 => SysNum::SetPGID,
            110 => SysNum::GetPPID,
            111 => SysNum::GetPGRP,
            158 => SysNum::ArchPRCTL,
            202 => SysNum::Futex,
            218 => SysNum::SetTIDAddress,
            231 => SysNum::ExitGroup,
            257 => SysNum::OpenAt,
            262 => SysNum::NewFstatAt,
            273 => SysNum::SetRobustList,
            293 => SysNum::Pipe2,
            302 => SysNum::PRLimit,
            318 => SysNum::GetRandom,
            334 => SysNum::Rseq,
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
            SysNum::LSeek => 8,
            SysNum::Mmap => 9,
            SysNum::Mprotect => 10,
            SysNum::Munmap => 11,
            SysNum::Brk => 12,
            SysNum::RTSigAction => 13,
            SysNum::RTSigProcmask => 14,
            SysNum::RTSigReturn => 15,
            SysNum::IOCtl => 16,
            SysNum::PRead => 17,
            SysNum::PWrite => 18,
            SysNum::Access => 21,
            SysNum::Dup => 32,
            SysNum::Dup2 => 33,
            SysNum::GetPID => 39,
            SysNum::Clone => 56,
            SysNum::Fork => 57,
            SysNum::VFork => 58,
            SysNum::Execve => 59,
            SysNum::Exit => 60,
            SysNum::Wait => 61,
            SysNum::Kill => 62,
            SysNum::Uname => 63,
            SysNum::Fcntl => 72,
            SysNum::Creat => 85,
            SysNum::SysInfo => 99,
            SysNum::GetUID => 102,
            SysNum::GetGID => 104,
            SysNum::SetUID => 105,
            SysNum::SetGID => 106,
            SysNum::GetEUID => 107,
            SysNum::GetEGID => 108,
            SysNum::SetPGID => 109,
            SysNum::GetPPID => 110,
            SysNum::GetPGRP => 111,
            SysNum::ArchPRCTL => 158,
            SysNum::Futex => 202,
            SysNum::SetTIDAddress => 218,
            SysNum::ExitGroup => 231,
            SysNum::OpenAt => 257,
            SysNum::NewFstatAt => 262,
            SysNum::SetRobustList => 273,
            SysNum::Pipe2 => 293,
            SysNum::PRLimit => 302,
            SysNum::GetRandom => 318,
            SysNum::Rseq => 334,
            SysNum::Other(num) => num,
        }
    }
}
