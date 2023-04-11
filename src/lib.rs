use fnv;
use std::{
    env,
    ptr,
    panic, 
    thread,
    process,
    time::Duration,
    io::{self, Result},
    hash::{Hash, Hasher},
};
use process_control::{ChildExt, Control};

const SUINDBOX: &str = "SUINDBOX";
const SUINDBOX_LENGTH: usize = 19; /* ':' plus 16 hexits */

pub fn fork<CHILD>(
    sandboxed: String,
    ser_module: String,
    ser_fn_map: String,
    timeout: Option<u64>,
    mem_limit: Option<usize>,
    in_child: CHILD,
) -> Result<process_control::Output> where CHILD: FnOnce() {

    let mut in_child = Some(in_child);
    fork_impl(sandboxed, 
              ser_module,
              ser_fn_map,
              timeout.unwrap_or(1000), 
              mem_limit.unwrap_or(1048576), 
              &mut || {
                in_child.take().unwrap()()
              })
}

fn fork_impl(
    sandboxed: String,
    ser_module: String,
    ser_fn_map: String,
    timeout: u64,
    mem_limit: usize,
    in_child: &mut dyn FnMut(),
) -> Result<process_control::Output> {

    let mut occurs = env::var(SUINDBOX).unwrap_or_else(|_| String::new());

    println!("[Parent] occurs = {}", occurs);

    if occurs.contains(&sandboxed) {
        println!("[Child]");
        match panic::catch_unwind(panic::AssertUnwindSafe(in_child)) {
            Ok(_) => process::exit(0),
            Err(_) => process::exit(70),
        }
    } else {
        println!("[Parent]");
        // Prevent misconfiguration creating a fork bomb
        if occurs.len() > 16 * SUINDBOX_LENGTH {
            panic!("festivities: Not forking due to >=16 levels of recursion");
        }

        occurs.push_str(&sandboxed);
        println!("[Parent] occurs = {}", occurs);

        println!("[Parent] current_exe = {:#?}", env::current_exe().unwrap());

        let mut command =
            process::Command::new(env::current_exe().expect("current_exe() failed, cannot fork"));

        command
            .arg(ser_module)
            .arg(ser_fn_map)
            .env(SUINDBOX, &occurs)
            .stdin(process::Stdio::null())
            .stdout(process::Stdio::piped())
            .stderr(process::Stdio::piped());

        let out = command
            .spawn()?
            .controlled_with_output()
            .time_limit(Duration::from_millis(timeout))
            .memory_limit(mem_limit)
            .terminate_for_timeout()
            .wait()?
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "Process timed out"))?;

        println!("[Parent] out = {:#?}", out);

        Ok(out)
    }
}