use sandbox;
use std::{
    io,
    env,
    ptr,
    panic, 
    thread,
    process,
    hash::Hash,
    time::Duration,
    collections::BTreeMap,
};
use sui_verifier::verifier as sui_bytecode_verifier;
use sui_types::{
    error::SuiError,
    // move_package::{FnInfo, FnInfoKey, FnInfoMap},
};
use serde::{Serialize, Deserialize};
use move_binary_format::file_format::CompiledModule;
use move_core_types::{account_address::AccountAddress};
use move_bytecode_verifier;
use std::path::Path;
use serde_json;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FnInfo {
    pub is_test: bool,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FnInfoKey {
    pub fn_name: String,
    pub mod_addr: AccountAddress,
}

pub type FnInfoMap = BTreeMap<FnInfoKey, FnInfo>;

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

    let pid = process::id();
    println!("Process ID is {}", pid);

    let args: Vec<String> = env::args().collect();
    let args_counter = args.len();

    match args_counter {
        4 => {
            println!("[sandbox] PARENT - move verifier");
        
            let arg1 = &args[1];
            let ser_comp_mod: Vec<u8> = serde_json::from_str(arg1).unwrap();
            let compiled_module: CompiledModule = CompiledModule::deserialize(&ser_comp_mod).unwrap();
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

            // println!("[sandbox] fork_id = {:#?}", fork_id!());

            let res = sandbox::fork(String::from("sandboxed"), arg1.to_string(), ser_fn_info_map, Some(time_limit_int), Some(mem_limit_int), || {
                move_bytecode_verifier::verify_module(&compiled_module).map_err(|err| {
                    SuiError::ModuleVerificationFailure {
                        error: err.to_string(),
                    }
                }).unwrap();
            })
            .unwrap();
            assert!(res.status.success());
            
            let outstr = String::from_utf8_lossy(&res.stdout);
            println!("[sandbox] OUTPUT = {}", outstr);
        }

        5 => {
            println!("[sandbox] PARENT - sui verifier");
        
            let arg1 = &args[1];
            let ser_comp_mod: Vec<u8> = serde_json::from_str(arg1).unwrap();
            let compiled_module: CompiledModule = CompiledModule::deserialize(&ser_comp_mod).unwrap();
            // println!("[sandbox] Compiled Module = {:#?}", compiled_module);

            let arg2 = &args[2];
            let fn_info_map: FnInfoMap = serde_json::from_str(arg2).unwrap();
            // println!("[sandbox] Function Map = {:#?}", fn_info_map);
            
            let arg3 = &args[3];
            let time_limit = String::from(arg3);
            let time_limit_int = time_limit.parse::<u64>().unwrap();
            // println!("[sandbox] Time Limit = {}", time_limit);
            
            let arg4 = &args[4];
            let mem_limit = String::from(arg4);
            let mem_limit_int = mem_limit.parse::<usize>().unwrap();
            // println!("[sandbox] Memory Limit = {}", mem_limit);

            // println!("[sandbox] fork_id = {:#?}", fork_id!());

            // let res = sandbox::fork(String::from("sandboxed"), arg1.to_string(), arg2.to_string(), Some(time_limit_int), Some(mem_limit_int), || {
            //     sui_bytecode_verifier::verify_module(&compiled_module, &fn_info_map).map_err(|err| {
            //         SuiError::ModuleVerificationFailure {
            //             error: err.to_string(),
            //         }
            //     }).unwrap();
            // })
            // .unwrap();
            // assert!(res.status.success());
            
            // let outstr = String::from_utf8_lossy(&res.stdout);
            // println!("[sandbox] OUTPUT = {}", outstr);
        }

        3 => {
            println!("[sandbox] CHILD");

            // println!("[sandbox] fork_id = {:#?}", fork_id!());

            let arg1 = &args[1];
            let ser_comp_mod: Vec<u8> = serde_json::from_str(arg1).unwrap();
            let compiled_module: CompiledModule = CompiledModule::deserialize(&ser_comp_mod).unwrap();
            // println!("[sandbox] Compiled Module = {:#?}", compiled_module);

            let arg2 = &args[2];
            if arg2 != ""{
                let fn_info_map: FnInfoMap = serde_json::from_str(arg2).unwrap();
                // println!("[sandbox] Function Map = {:#?}", fn_info_map);

                // let res = sandbox::fork(fork_id!(), arg1.to_string(), arg2.to_string(), Some(time_limit_int), Some(mem_limit_int), || {
                //     sui_bytecode_verifier::verify_module(&compiled_module, &fn_info_map).map_err(|err| {
                //         SuiError::ModuleVerificationFailure {
                //             error: err.to_string(),
                //         }
                //     }).unwrap();
                // })
                // .unwrap();
                // assert!(res.status.success());
                
                // let outstr = String::from_utf8_lossy(&res.stdout);
                // println!("[sandbox] OUTPUT = {}", outstr);
            }
            else { 
                println!("[sandbox] Child calling move verifier");
                let res = sandbox::fork(String::from(""), String::from(""), String::from(""), None, None, || {
                    move_bytecode_verifier::verify_module(&compiled_module).map_err(|err| {
                        SuiError::ModuleVerificationFailure {
                            error: err.to_string(),
                        }
                    }).unwrap();
                })
                .unwrap();
                assert!(res.status.success());
                
                let outstr = String::from_utf8_lossy(&res.stdout);
                println!("[sandbox] OUTPUT = {}", outstr);
            }
        }   
    
        _ => {
            println!("[sandbox] Invalid number of arguments!");
        }
    
    }
}