use nix::libc::{
    CLOCK_BOOTTIME, CLOCK_BOOTTIME_ALARM, CLOCK_MONOTONIC, CLOCK_MONOTONIC_COARSE,
    CLOCK_MONOTONIC_RAW, CLOCK_PROCESS_CPUTIME_ID, CLOCK_REALTIME, CLOCK_REALTIME_ALARM,
    CLOCK_REALTIME_COARSE, CLOCK_TAI, CLOCK_THREAD_CPUTIME_ID,
};
use serde::Serialize;
use tracing::warn;

macro_rules! sys_num {
    (
        $($variant:ident => $val:expr,)+
    ) => {
        #[derive(Copy, Clone, Debug)]
        pub enum SysNum {
            $($variant,)+
            Other(u64),
        }

        impl From<u64> for SysNum {
            fn from(num: u64) -> Self {
                match num {
                    $($val => SysNum::$variant,)+
                    _ => Self::Other(num),
                }
            }
        }

        impl From<SysNum> for u64 {
            fn from(num: SysNum) -> u64 {
                match num {
                    $(SysNum::$variant => $val,)+
                    SysNum::Other(num) => num,
                }
            }
        }
    }
}

sys_num!(
    Read => 0,
    Write => 1,
    Open => 2,
    Close => 3,
    Stat => 4,
    FStat => 5,
    LStat => 6,
    LSeek => 8,
    Mmap => 9,
    Mprotect => 10,
    Munmap => 11,
    Brk => 12,
    RTSigAction => 13,
    RTSigProcmask => 14,
    RTSigReturn => 15,
    IOCtl => 16,  // TODO?
    PRead => 17,
    PWrite => 18,
    Access => 21,
    MAdvise => 28,
    Dup => 32,
    Dup2 => 33,
    Nanosleep => 35,
    GetPID => 39,
    Socket => 41,  // TODO?
    Connect => 42,  // TODO?
    GetPeerName => 52,  // TODO?
    Clone => 56,
    Fork => 57,
    VFork => 58,
    Execve => 59,
    Exit => 60,
    Wait => 61,
    Kill => 62,  // TODO?
    Uname => 63,  // TODO?
    Fcntl => 72,
    GetCWD => 79,  // TODO?
    Chdir => 80,  // TODO?
    Creat => 85,
    ReadLink => 89,
    GetRLimit => 97,
    SysInfo => 99,  // TODO?
    GetUID => 102,
    GetGID => 104,
    SetUID => 105,
    SetGID => 106,
    GetEUID => 107,
    GetEGID => 108,
    SetPGID => 109,
    GetPPID => 110,
    GetPGRP => 111,
    SigAltStack => 131,
    StatFS => 137,
    FStatFS => 138,
    ArchPRCTL => 158,
    GetTID => 186,
    GetXAttr => 191,
    LGetXAttr => 192,
    Time => 201,
    Futex => 202,
    SchedSetAffinity => 203,
    SchedGetAffinity => 204,
    GetDEnts => 217,  // TODO?
    SetTIDAddress => 218,
    FAdvise => 221,
    ClockGetTime => 228,
    ExitGroup => 231,
    OpenAt => 257,
    NewFstatAt => 262,
    SetRobustList => 273,
    UTimeNsAt => 280,
    Pipe2 => 293,
    PRLimit => 302,
    GetRandom => 318,
    StatX => 332,
    Rseq => 334,
    FAccessAt2 => 439,
);

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
