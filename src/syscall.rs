use nix::libc::{
    CLOCK_BOOTTIME, CLOCK_BOOTTIME_ALARM, CLOCK_MONOTONIC, CLOCK_MONOTONIC_COARSE,
    CLOCK_MONOTONIC_RAW, CLOCK_PROCESS_CPUTIME_ID, CLOCK_REALTIME, CLOCK_REALTIME_ALARM,
    CLOCK_REALTIME_COARSE, CLOCK_TAI, CLOCK_THREAD_CPUTIME_ID,
};
use serde::Serialize;
use tracing::warn;

#[derive(Copy, Clone, Debug)]
pub enum SysNum {
    Access,
    ArchPRCTL,
    Brk,
    Chdir, // TODO?
    ClockGetTime,
    Clone,
    Close,
    Connect, // TODO?
    Creat,
    Dup,
    Dup2,
    Execve,
    Exit,
    ExitGroup,
    FAccessAt2,
    FAdvise,
    Fcntl,
    Fork,
    Futex,
    FStat,
    FStatFS,
    GetCWD,   // TODO?
    GetDEnts, // TODO?
    GetEGID,
    GetEUID,
    GetGID,
    GetPeerName, // TODO?
    GetPGRP,
    GetPID,
    GetPPID,
    GetRandom,
    GetUID,
    GetXAttr,
    IOCtl, // TODO?
    Kill,  // TOOD?
    LGetXAttr,
    LSeek,
    LStat,
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
    ReadLink,
    Rseq,
    RTSigAction,
    RTSigProcmask,
    RTSigReturn,
    SetGID,
    SetPGID,
    SetRobustList,
    SetTIDAddress,
    SetUID,
    Socket, // TODO?
    Stat,
    StatFS,
    StatX,
    SysInfo, // TODO?
    Uname,   // TODO?
    VFork,
    Wait,
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
            4 => SysNum::Stat,
            5 => SysNum::FStat,
            6 => SysNum::LStat,
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
            41 => SysNum::Socket,
            42 => SysNum::Connect,
            52 => SysNum::GetPeerName,
            56 => SysNum::Clone,
            57 => SysNum::Fork,
            58 => SysNum::VFork,
            59 => SysNum::Execve,
            60 => SysNum::Exit,
            61 => SysNum::Wait,
            62 => SysNum::Kill,
            63 => SysNum::Uname,
            72 => SysNum::Fcntl,
            79 => SysNum::GetCWD,
            80 => SysNum::Chdir,
            85 => SysNum::Creat,
            89 => SysNum::ReadLink,
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
            137 => SysNum::StatFS,
            138 => SysNum::FStatFS,
            158 => SysNum::ArchPRCTL,
            191 => SysNum::GetXAttr,
            192 => SysNum::LGetXAttr,
            202 => SysNum::Futex,
            217 => SysNum::GetDEnts,
            218 => SysNum::SetTIDAddress,
            221 => SysNum::FAdvise,
            228 => SysNum::ClockGetTime,
            231 => SysNum::ExitGroup,
            257 => SysNum::OpenAt,
            262 => SysNum::NewFstatAt,
            273 => SysNum::SetRobustList,
            293 => SysNum::Pipe2,
            302 => SysNum::PRLimit,
            318 => SysNum::GetRandom,
            332 => SysNum::StatX,
            334 => SysNum::Rseq,
            439 => SysNum::FAccessAt2,
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
            SysNum::Stat => 4,
            SysNum::FStat => 5,
            SysNum::LStat => 6,
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
            SysNum::Socket => 41,
            SysNum::Connect => 42,
            SysNum::GetPeerName => 52,
            SysNum::Clone => 56,
            SysNum::Fork => 57,
            SysNum::VFork => 58,
            SysNum::Execve => 59,
            SysNum::Exit => 60,
            SysNum::Wait => 61,
            SysNum::Kill => 62,
            SysNum::Uname => 63,
            SysNum::Fcntl => 72,
            SysNum::GetCWD => 79,
            SysNum::Chdir => 80,
            SysNum::Creat => 85,
            SysNum::ReadLink => 89,
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
            SysNum::StatFS => 137,
            SysNum::FStatFS => 138,
            SysNum::ArchPRCTL => 158,
            SysNum::GetXAttr => 191,
            SysNum::LGetXAttr => 192,
            SysNum::Futex => 202,
            SysNum::GetDEnts => 217,
            SysNum::SetTIDAddress => 218,
            SysNum::FAdvise => 221,
            SysNum::ClockGetTime => 228,
            SysNum::ExitGroup => 231,
            SysNum::OpenAt => 257,
            SysNum::NewFstatAt => 262,
            SysNum::SetRobustList => 273,
            SysNum::Pipe2 => 293,
            SysNum::PRLimit => 302,
            SysNum::GetRandom => 318,
            SysNum::StatX => 332,
            SysNum::Rseq => 334,
            SysNum::FAccessAt2 => 439,
            SysNum::Other(num) => num,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum Clock {
    Realtime(i32),
    Monotonic(i32),
    ProcessCPUTime,
    ThreadCPUTime,
    Other(i32),
}

impl From<u64> for Clock {
    fn from(num: u64) -> Self {
        let num = num as i32;
        match num {
            CLOCK_REALTIME | CLOCK_REALTIME_COARSE | CLOCK_REALTIME_ALARM => Clock::Realtime(num),
            CLOCK_MONOTONIC
            | CLOCK_MONOTONIC_COARSE
            | CLOCK_MONOTONIC_RAW
            | CLOCK_BOOTTIME
            | CLOCK_BOOTTIME_ALARM => Clock::Monotonic(num),
            CLOCK_PROCESS_CPUTIME_ID => Clock::ProcessCPUTime,
            CLOCK_THREAD_CPUTIME_ID => Clock::ThreadCPUTime,
            CLOCK_TAI => {
                warn!("Unknown clock type: {}", num);
                Clock::Other(num)
            }
            _ => {
                warn!("Unknown clock type: {}", num);
                Clock::Other(num)
            }
        }
    }
}
