use sundbox;
use std::{
    io,
    env,
    ptr,
    panic, 
    thread,
    process,
    error::Error,
    time::Duration,
};
use std::collections::HashMap;
use sui_verifier::verifier as sui_bytecode_verifier;
use sui_types::{
    error::SuiError,
    move_package::{FnInfo, FnInfoKey, FnInfoMap},
};
use serde_json_any_key::*;
use move_binary_format::file_format::CompiledModule;
use move_bytecode_verifier;
use serde_json;

pub fn get_mem_limit() {
	unsafe {
		let mut rlim = libc::rlimit{rlim_cur: 0, rlim_max: 0};
		if libc::getrlimit(libc::RLIMIT_AS, &mut rlim) != 0 {
			let err = io::Error::last_os_error();
			panic!("raise_fd_limit: error calling getrlimit: {}", err);
		}

        println!("Hard Limit: {}",rlim.rlim_max);
        println!("Soft Limit: {}",rlim.rlim_cur);
	}
}

fn _sandboxed_function() {

    println!("[*] Running inside the Sandbox");
    
    get_mem_limit();

    // Example how to trigger the memory limit exception
    let page_size = 1048576*0x1 as usize;
    let mut addr: *mut libc::c_void = ptr::null_mut();
    let prot = libc::PROT_READ | libc::PROT_WRITE;
    let flags = libc::MAP_PRIVATE | libc::MAP_ANONYMOUS;
    let fd = -1;
    let offset = 0;

    unsafe {
        addr = libc::mmap(
            ptr::null_mut(),
            page_size,
            prot,
            flags,
            fd,
            offset,
        );
    }

    if addr == libc::MAP_FAILED {
        panic!("Failed to create memory mapping");
    }

    // Example how to trigger the time limit exception
    thread::sleep(Duration::from_secs(1));

}

fn main() {

    let args: Vec<String> = env::args().collect();
    let args_counter = args.len();

    match args_counter {
        4 => {
            // println!("[sandbox] PARENT - move verifier");
        
            let arg1 = &args[1];
            let ser_comp_mod: Vec<u8> = serde_json::from_str(arg1).unwrap();
            let compiled_module = CompiledModule::deserialize(&ser_comp_mod).unwrap();
            // println!("[sandbox] Compiled Module = {:#?}", compiled_module);

            let ser_fn_info_map: String = String::from("");
            
            let arg2 = &args[2];
            let time_limit = String::from(arg2);
            let time_limit_int = time_limit.parse::<u64>().unwrap();
            // println!("[sandbox] Time Limit = {}", time_limit);
            
            let arg3 = &args[3];
            let mem_limit = String::from(arg3);
            let mem_limit_int = mem_limit.parse::<usize>().unwrap();
            // println!("[sandbox] Memory Limit = {}", mem_limit);

            let sundbox_result = sundbox::fork(String::from("sandboxed"), arg1.to_string(), ser_fn_info_map, Some(time_limit_int), Some(mem_limit_int), || {
                move_bytecode_verifier::verify_module(&compiled_module).map_err(|err| {
                    SuiError::ModuleVerificationFailure {
                        error: err.to_string(),
                    }
                }).unwrap();
            }).unwrap();

            
            let sunbox_stdout = String::from_utf8_lossy(&sundbox_result.stdout);
            let sunbox_stderr = String::from_utf8_lossy(&sundbox_result.stderr);

            println!("{:?}", sunbox_stdout.trim().replace("\"", ""));
            eprintln!("{:?}", sunbox_stderr.trim().replace("\"", ""));
            process::exit(0)
            
        }

        5 => {
            // println!("[sandbox] PARENT - sui verifier");
        
            let arg1 = &args[1];
            let ser_comp_mod: Vec<u8> = serde_json::from_str(arg1).unwrap();
            let compiled_module = CompiledModule::deserialize(&ser_comp_mod).unwrap();
            // println!("[sandbox] Compiled Module = {:#?}", compiled_module);

            let mut input = String::new();
            io::stdin().read_line(&mut input).expect("Failed to read line");
            let fn_info_hmap : HashMap<FnInfoKey, FnInfo> = json_to_map(&input).unwrap();
            let fn_info_bmap : FnInfoMap = fn_info_hmap.into_iter()
                .map(|(key, value)| (FnInfoKey::from(key), FnInfo::from(value)))
                .collect();
            // println!("[sandbox] Function Map = {:#?}", fn_info_bmap);

            let arg3 = &args[3];
            let time_limit = String::from(arg3);
            let time_limit_int = time_limit.parse::<u64>().unwrap();
            // println!("[sandbox] Time Limit = {}", time_limit);
            
            let arg4 = &args[4];
            let mem_limit = String::from(arg4);
            let mem_limit_int = mem_limit.parse::<usize>().unwrap();
            // println!("[sandbox] Memory Limit = {}", mem_limit);

            let sundbox_result = sundbox::fork(String::from("sandboxed"), arg1.to_string(), input, Some(time_limit_int), Some(mem_limit_int), || {
                sui_bytecode_verifier::verify_module(&compiled_module, &fn_info_bmap).map_err(|err| {
                    SuiError::ModuleVerificationFailure {
                        error: err.to_string(),
                    }
                }).unwrap();
            }).unwrap();
            
            let sunbox_stdout = String::from_utf8_lossy(&sundbox_result.stdout);
            let sunbox_stderr = String::from_utf8_lossy(&sundbox_result.stderr);

            println!("{:?}", sunbox_stdout.trim().replace("\"", ""));
            eprintln!("{:?}", sunbox_stderr.trim().replace("\"", ""));
            process::exit(0)

        }

        2 => {
            // println!("[sandbox] CHILD");

            let arg1 = &args[1];
            let ser_comp_mod: Vec<u8> = serde_json::from_str(arg1).unwrap();
            let compiled_module = CompiledModule::deserialize(&ser_comp_mod).unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).expect("Failed to read line");

            if input != "" {
                // println!("[sandbox] CHILD - sui verifier");

                let fn_info_hmap : HashMap<FnInfoKey, FnInfo> = json_to_map(&input).unwrap();
                let fn_info_bmap : FnInfoMap = fn_info_hmap.into_iter()
                    .map(|(key, value)| (FnInfoKey::from(key), FnInfo::from(value)))
                    .collect();
                // println!("[sandbox] Function Map = {:#?}", fn_info_bmap);

                let sundbox_result = sundbox::fork(String::from("sandboxed"), arg1.to_string(), input, None, None, || {
                    match sui_bytecode_verifier::verify_module(&compiled_module, &fn_info_bmap) {
                        Ok(_) => (),
                        Err(err) => { 
                            eprintln!("{:?}", err.source().as_ref().unwrap());
                        }
                    };
                }).unwrap();

                let sunbox_stdout = String::from_utf8_lossy(&sundbox_result.stdout);
                let sunbox_stderr = String::from_utf8_lossy(&sundbox_result.stderr);
    
                println!("{:?}", sunbox_stdout.trim().replace("\"", ""));
                eprintln!("{:?}", sunbox_stderr.trim().replace("\"", ""));
                process::exit(0)

            }
            else { 
                // println!("[sandbox] CHILD - move verifier");

                let sundbox_result = sundbox::fork(String::from(""), String::from(""), String::from(""), None, None, || {
                    match move_bytecode_verifier::verify_module(&compiled_module) {
                        Ok(_) => (),
                        Err(err) => {
                            eprintln!("{:?}", err.source().unwrap());
                        }
                    };
                }).unwrap();
                
                let sunbox_stdout = String::from_utf8_lossy(&sundbox_result.stdout);
                let sunbox_stderr = String::from_utf8_lossy(&sundbox_result.stderr);
    
                println!("{:?}", sunbox_stdout.trim().replace("\"", ""));
                eprintln!("{:?}", sunbox_stderr.trim().replace("\"", ""));
                process::exit(0)

            }
        }   
    
        _ => {
            eprintln!("[sundbox] Invalid number of arguments!");
            process::exit(70)
        }
    
    }
}