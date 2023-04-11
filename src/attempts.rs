
use std::io;
use std::time::Duration;
// use process_control::ChildExt;
// use process_control::Control;
use String;
use libc::{self, pid_t};
use std::error::Error;
// use std::os::unix::process::Command;
use std::process;
use tokio::net::unix::pipe;

// macro_rules! if_memory_limit {
//     ( $($item:item)+ ) => {
//         $(
//             #[cfg(process_control_memory_limit)]
//             $item
//         )+
//     };
// }

// if_memory_limit! {
//     use std::convert::TryFrom;
//     use std::ptr;

//     use libc::rlimit;
//     use libc::RLIMIT_AS;
// }

// #[derive(Debug)]
// struct RawPid(pid_t);

// impl RawPid {
//     fn new(process: &Child) -> Self {
//         let pid: u32 = process.id();
//         Self(pid.try_into().expect("process identifier is invalid"))
//     }

//     if_waitid! {
//         const fn as_id(&self) -> id_t {
//             static_assert!(pid_t::MAX == i32::MAX);
//             static_assert!(
//                 mem::size_of::<pid_t>() <= mem::size_of::<id_t>(),
//             );

//             self.0 as _
//         }
//     }
// }

// #[derive(Debug)]
// pub struct Sandbox<'a> {
//     process: &'a mut Child,
//     pid: RawPid,
// }

// impl<'a> Handle<'a> {
//     pub fn new(process: &'a mut Child) -> Self {
//         Self {
//             pid: RawPid::new(process),
//             process,
//         }
//     }

//     if_memory_limit! {
//         fn set_limit(
//             &mut self,
//             resource: LimitResource,
//             limit: usize,
//         ) -> io::Result<()> {
//             #[cfg(target_pointer_width = "32")]
//             type PointerWidth = u32;
//             #[cfg(target_pointer_width = "64")]
//             type PointerWidth = u64;
//             #[cfg(not(any(
//                 target_pointer_width = "32",
//                 target_pointer_width = "64",
//             )))]
//             compile_error!("unsupported pointer width");

//             #[cfg_attr(
//                 not(target_os = "freebsd"),
//                 allow(clippy::useless_conversion)
//             )]
//             let limit = PointerWidth::try_from(limit)
//                 .expect("`usize` too large for pointer width")
//                 .into();

//             check_syscall(unsafe {
//                 libc::prlimit(
//                     self.pid.0,
//                     resource,
//                     &rlimit {
//                         rlim_cur: limit,
//                         rlim_max: limit,
//                     },
//                     ptr::null_mut(),
//                 )
//             })
//         }

//         pub(super) fn set_memory_limit(
//             &mut self,
//             limit: usize,
//         ) -> io::Result<()> {
//             self.set_limit(RLIMIT_AS, limit)
//         }
//     }

//     pub(super) fn wait(
//         &mut self,
//         time_limit: Option<Duration>,
//     ) -> WaitResult<ExitStatus> {
//         wait::wait(self, time_limit)
//     }
// }

macro_rules! rtassert {
    ( $arg:expr ) => ( {
        if !$arg {
            rtabort!(" assertion failed: {}", stringify!($arg));
        }
    } )
}


fn abort() {
    process::abort();
}

macro_rules! rtabort {
    ($($arg:tt)*) => ( {
        abort();
    } )
}

pub fn check_result(result: i32) -> crate::io::Result<i32> {
    if result == -1 { 
        println!("All bad");
        Err(crate::io::Error::last_os_error()) 
    } else { 
        println!("All good");
        Ok(result) 
    }
}

unsafe fn do_fork() -> Result<pid_t, io::Error> {

    check_result(libc::fork()).map(|res| (res))

}

fn verifier() -> Result<(), io::Error> {
    println!("Inside the verifier baby!!!");
    Err(io::Error::last_os_error())
}

const CLOEXEC_MSG_FOOTER: [u8; 4] = *b"NOEX";

// unix process https://github.com/rust-lang/rust/blob/864b6258fc7b493aec01f980b31ff23901c0edae/library/std/src/sys/unix/process/process_unix.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {

    let (input, output) = pipe::anon_pipe()?;

    let pid: pid_t = unsafe { do_fork()? };

    if pid == 0 {

        drop(input);

        println!("  [C] Here in the child");

        let Err(err) = unsafe { verifier() };
        let errno = err.raw_os_error().unwrap_or(libc::EINVAL) as u32;
        let errno = errno.to_be_bytes();
        let bytes = [
            errno[0],
            errno[1],
            errno[2],
            errno[3],
            CLOEXEC_MSG_FOOTER[0],
            CLOEXEC_MSG_FOOTER[1],
            CLOEXEC_MSG_FOOTER[2],
            CLOEXEC_MSG_FOOTER[3],
        ];
        // pipe I/O up to PIPE_BUF bytes should be atomic, and then
        // we want to be sure we *don't* run at_exit destructors as
        // we're being torn down regardless
        rtassert!(output.write(&bytes).is_ok());


        unsafe { libc::_exit(1) }
    }

    drop(output);
    let mut bytes = [0; 8];

    loop {
        println!("[P] Here in parent");
        match input.read(&mut bytes) {
            Ok(0) => return Ok(()),
            Ok(8) => {
                let (errno, footer) = bytes.split_at(4);
                assert_eq!(
                    CLOEXEC_MSG_FOOTER, footer,
                    "Validation on the CLOEXEC pipe failed: {:?}",
                    bytes
                );
                let errno = i32::from_be_bytes(errno.try_into().unwrap());
                // assert!(p.wait().is_ok(), "wait() should either return Ok or panic");
                println!("Eroare {}", errno);
            }
            // Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            // Err(e) => {
            //     assert!(p.wait().is_ok(), "wait() should either return Ok or panic");
            //     panic!("the CLOEXEC pipe failed: {e:?}")
            // }
            // Ok(..) => {
            //     // pipe I/O up to PIPE_BUF bytes should be atomic
            //     assert!(p.wait().is_ok(), "wait() should either return Ok or panic");
            //     panic!("short read on the CLOEXEC pipe")
            // }
        }

    }

    


    // // Safety: We obtained the pidfd from calling `clone3` with
    // // `CLONE_PIDFD` so it's valid an otherwise unowned.
    // let mut p = unsafe { Process::new(pid, pidfd) };
    // let mut bytes = [0; 8];

    // // loop to handle EINTR
    // loop {
    //     match input.read(&mut bytes) {
    //         Ok(0) => return Ok((p, ours)),
    //         Ok(8) => {
    //             let (errno, footer) = bytes.split_at(4);
    //             assert_eq!(
    //                 CLOEXEC_MSG_FOOTER, footer,
    //                 "Validation on the CLOEXEC pipe failed: {:?}",
    //                 bytes
    //             );
    //             let errno = i32::from_be_bytes(errno.try_into().unwrap());
    //             assert!(p.wait().is_ok(), "wait() should either return Ok or panic");
    //             return Err(Error::from_raw_os_error(errno));
    //         }
    //         Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
    //         Err(e) => {
    //             assert!(p.wait().is_ok(), "wait() should either return Ok or panic");
    //             panic!("the CLOEXEC pipe failed: {e:?}")
    //         }
    //         Ok(..) => {
    //             // pipe I/O up to PIPE_BUF bytes should be atomic
    //             assert!(p.wait().is_ok(), "wait() should either return Ok or panic");
    //             panic!("short read on the CLOEXEC pipe")
    //         }
    //     }
    // }

}

