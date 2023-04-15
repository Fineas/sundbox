use std::{
    env,
    panic, 
    process,
    time::Duration,
    io::{self, Result, Write},
};
use process_control::{ChildExt, Control};

const SUNDBOX: &str = "SUNDBOX";
const SUNDBOX_LENGTH: usize = 19; /* ':' plus 16 hexits */

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

    let mut occurs = env::var(SUNDBOX).unwrap_or_else(|_| String::new());

    if occurs.contains(&sandboxed) {
        // println!("[Child]");
        match panic::catch_unwind(panic::AssertUnwindSafe(in_child)) {
            Ok(_) => process::exit(0),
            Err(_) => process::exit(70),
        }
    } else {
        // println!("[Parent]");
        if occurs.len() > 16 * SUNDBOX_LENGTH {
            panic!("Not forking due to >=16 levels of recursion");
        }

        occurs.push_str(&sandboxed);

        let mut sundbox =
            process::Command::new(env::current_exe().expect("current_exe() failed, cannot fork"))
                .arg(ser_module)
                // .arg(ser_fn_map)
                .env(SUNDBOX, &occurs)
                .stdin(process::Stdio::piped())
                .stdout(process::Stdio::piped())
                .stderr(process::Stdio::piped())
                .spawn()
                .expect("failed to execute process");

        let sundbox_stdin = sundbox.stdin.as_mut().unwrap();
        sundbox_stdin.write_all(ser_fn_map.as_bytes()).expect("failed to write to stdin");
        drop(sundbox_stdin);

        let sundbox = sundbox.controlled_with_output()
                                 .time_limit(Duration::from_millis(timeout))
                                 .memory_limit(mem_limit)
                                 .terminate_for_timeout()
                                 .wait()?
                                 .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "Process timed out"))?;

        // println!("[Parent] out = {:#?}", sundbox);
        
        if sundbox.status.success() {
            println!("{:#?}", String::from_utf8_lossy(&sundbox.stdout));
            println!("Verifier succeeded!");
            Ok(sundbox)
        }
        else {
            eprintln!("{:#?}", String::from_utf8_lossy(&sundbox.stderr));
            eprintln!("Verifier threw Error");
            Ok(sundbox)
        }
    }
}